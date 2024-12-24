use crate::OuterContext;

use sd_core_file_helper::IsolatedFilePathData;
use sd_core_shared_types::cas_id::CasId;

use sd_file_ext::{extensions::Extension, kind::ObjectKind};
use sd_prisma::prisma::{device, file_path, location};
use sd_task_system::{TaskDispatcher, TaskHandle};
use sd_utils::error::FileIOError;

use std::{
	collections::{hash_map::Entry, HashMap},
	fs::Metadata,
	mem,
	path::Path,
	sync::Arc,
};

use tokio::fs;
use tracing::trace;

mod cas_id;
pub mod job;
mod shallow;
mod tasks;

pub use cas_id::generate_cas_id;

pub use job::FileIdentifier;
pub use shallow::shallow;

use tasks::FilePathToCreateOrLinkObject;

// we break these tasks into chunks of 100 to improve performance
const CHUNK_SIZE: usize = 100;

#[derive(Debug, Clone)]
pub struct FileMetadata {
	pub cas_id: Option<CasId<'static>>,
	pub kind: ObjectKind,
	pub fs_metadata: Metadata,
}

impl FileMetadata {
	/// Fetch metadata from the file system and generate a cas id for the file
	/// if it's not empty.
	///
	/// # Panics
	/// Will panic if the file is a directory.
	pub async fn new(
		location_path: impl AsRef<Path> + Send,
		iso_file_path: &IsolatedFilePathData<'_>,
	) -> Result<Self, FileIOError> {
		let path = location_path.as_ref().join(iso_file_path);

		let fs_metadata = fs::metadata(&path)
			.await
			.map_err(|e| FileIOError::from((&path, e)))?;

		if fs_metadata.is_dir() {
			trace!(path = %path.display(), "Skipping directory;");
			return Ok(Self {
				cas_id: None,
				kind: ObjectKind::Folder,
				fs_metadata,
			});
		}

		// derive Object kind
		let kind = Extension::resolve_conflicting(&path, false)
			.await
			.map_or(ObjectKind::Unknown, Into::into);

		let cas_id = if fs_metadata.len() != 0 {
			generate_cas_id(&path, fs_metadata.len())
				.await
				.map_err(|e| FileIOError::from((&path, e)))?
		} else {
			// We can't do shit with empty files
			trace!(path = %path.display(), %kind, "Skipping empty file;");
			return Ok(Self {
				cas_id: None,
				kind,
				fs_metadata,
			});
		};

		trace!(
			path = %path.display(),
			?cas_id,
			%kind,
			"Analyzed file;",
		);

		Ok(Self {
			cas_id: Some(cas_id),
			kind,
			fs_metadata,
		})
	}
}

fn orphan_path_filters_shallow(
	location_id: location::id::Type,
	file_path_id: Option<file_path::id::Type>,
	sub_iso_file_path: &IsolatedFilePathData<'_>,
) -> Vec<file_path::WhereParam> {
	sd_utils::chain_optional_iter(
		[
			file_path::object_id::equals(None),
			file_path::location_id::equals(Some(location_id)),
			file_path::materialized_path::equals(Some(
				sub_iso_file_path
					.materialized_path_for_children()
					.expect("sub path for shallow identifier must be a directory"),
			)),
		],
		[file_path_id.map(file_path::id::gt)],
	)
}

fn orphan_path_filters_deep(
	location_id: location::id::Type,
	file_path_id: Option<file_path::id::Type>,
	maybe_sub_iso_file_path: Option<&IsolatedFilePathData<'_>>,
) -> Vec<file_path::WhereParam> {
	sd_utils::chain_optional_iter(
		[
			file_path::object_id::equals(None),
			file_path::location_id::equals(Some(location_id)),
		],
		[
			// this is a workaround for the cursor not working properly
			file_path_id.map(file_path::id::gt),
			maybe_sub_iso_file_path.as_ref().map(|sub_iso_file_path| {
				file_path::materialized_path::starts_with(
					sub_iso_file_path
						.materialized_path_for_children()
						.expect("sub path iso_file_path must be a directory"),
				)
			}),
		],
	)
}

async fn dispatch_object_processor_tasks<Iter, Dispatcher>(
	file_paths_by_cas_id: Iter,
	ctx: &impl OuterContext,
	device_id: device::id::Type,
	dispatcher: &Dispatcher,
	with_priority: bool,
) -> Result<Vec<TaskHandle<crate::Error>>, Dispatcher::DispatchError>
where
	Iter: IntoIterator<Item = (CasId<'static>, Vec<FilePathToCreateOrLinkObject>)> + Send,
	Iter::IntoIter: Send,
	Dispatcher: TaskDispatcher<crate::Error>,
{
	let mut current_batch = HashMap::<_, Vec<_>>::new();
	let mut tasks = vec![];

	let mut current_batch_size = 0;

	for (cas_id, objects_to_create_or_link) in file_paths_by_cas_id {
		if objects_to_create_or_link.len() >= CHUNK_SIZE {
			tasks.push(
				dispatcher
					.dispatch(tasks::ObjectProcessor::new(
						HashMap::from([(cas_id, objects_to_create_or_link)]),
						Arc::clone(ctx.db()),
						ctx.sync().clone(),
						device_id,
						with_priority,
					))
					.await?,
			);
		} else {
			current_batch_size += objects_to_create_or_link.len();
			match current_batch.entry(cas_id) {
				Entry::Occupied(entry) => {
					entry.into_mut().extend(objects_to_create_or_link);
				}
				Entry::Vacant(entry) => {
					entry.insert(objects_to_create_or_link);
				}
			}

			if current_batch_size >= CHUNK_SIZE {
				tasks.push(
					dispatcher
						.dispatch(tasks::ObjectProcessor::new(
							mem::take(&mut current_batch),
							Arc::clone(ctx.db()),
							ctx.sync().clone(),
							device_id,
							with_priority,
						))
						.await?,
				);

				current_batch_size = 0;
			}
		}
	}

	if !current_batch.is_empty() {
		tasks.push(
			dispatcher
				.dispatch(tasks::ObjectProcessor::new(
					current_batch,
					Arc::clone(ctx.db()),
					ctx.sync().clone(),
					device_id,
					with_priority,
				))
				.await?,
		);
	}

	Ok(tasks)
}

fn accumulate_file_paths_by_cas_id(
	input: HashMap<CasId<'static>, Vec<FilePathToCreateOrLinkObject>>,
	accumulator: &mut HashMap<CasId<'static>, Vec<FilePathToCreateOrLinkObject>>,
) {
	for (cas_id, file_paths) in input {
		match accumulator.entry(cas_id) {
			Entry::<_, Vec<_>>::Occupied(entry) => {
				entry.into_mut().extend(file_paths);
			}
			Entry::Vacant(entry) => {
				entry.insert(file_paths);
			}
		}
	}
}
