#![recursion_limit = "256"]
#![warn(clippy::unwrap_used, clippy::panic)]

mod config;
mod hardware;
mod platform;

pub use config::*;
pub use hardware::*;
pub use platform::*;

// Core crates
use sd_core_cloud_services::CloudServices;
use sd_core_cloud_services::AUTH_SERVER_URL;
use sd_core_library::{Libraries, LibraryManagerError};
use sd_core_location::{LocationManagerError, Locations};
use sd_core_location_scan::{media_processor::ThumbnailKind, JobSystem};
use sd_core_prisma_helpers::CasId;
use sd_core_shared_types::notification::Notifications;
use sd_core_volume::{VolumeManagerActor, Volumes};

// Shared crates
use sd_crypto::CryptoRng;
use sd_task_system::TaskSystem;
use sd_utils::error::FileIOError;

// Dependencies
use chrono::{DateTime, Utc};
use futures_concurrency::future::Join;
use std::{
	fmt,
	path::{Path, PathBuf},
	sync::Arc,
};
use thiserror::Error;
use tokio::{
	fs, io,
	sync::{broadcast, Mutex},
};
use tracing::{error, info, warn};
use tracing_appender::{
	non_blocking::{NonBlocking, WorkerGuard},
	rolling::{RollingFileAppender, Rotation},
};
use tracing_subscriber::{
	filter::FromEnvError, fmt::format::Format, prelude::*, registry, EnvFilter,
};

// Local module imports
use crate::api::{CoreEvent, Router};
use api::notifications::{Notification, NotificationData, NotificationId};
use context::{JobContext, NodeContext};
use notifications::Notifications;
/// Represents a single running instance of the Spacedrive core.
/// Holds references to all the services that make up the Spacedrive core.
pub struct Node {
	pub data_dir: PathBuf,
	pub config: Arc<config::Manager>,
	pub libraries: Arc<Libraries>,
	pub volumes: Arc<Volumes>,
	pub locations: Arc<Locations>,
	// pub p2p: Arc<p2p::P2PManager>,
	pub event_bus: (broadcast::Sender<CoreEvent>, broadcast::Receiver<CoreEvent>),
	pub notifications: Notifications,
	pub task_system: TaskSystem<sd_core_heavy_lifting::Error>,
	pub job_system: JobSystem<NodeContext, JobContext<NodeContext>>,
	pub cloud_services: Arc<CloudServices>,
	/// This should only be used to generate the seed of local instances of [`CryptoRng`].
	/// Don't use this as a common RNG, it will fuck up Core's performance due to this Mutex.
	pub master_rng: Arc<Mutex<CryptoRng>>,
}

impl fmt::Debug for Node {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Node")
			.field("data_dir", &self.data_dir)
			.finish()
	}
}

impl Node {
	pub async fn new(data_dir: impl AsRef<Path>) -> Result<(Arc<Node>, Arc<Router>), NodeError> {
		let data_dir = data_dir.as_ref();

		info!(data_directory = %data_dir.display(), "Starting core;");

		#[cfg(debug_assertions)]
		let init_data = util::debug_initializer::InitConfig::load(data_dir).await?;

		// This error is ignored because it's throwing on mobile despite the folder existing.
		let _ = fs::create_dir_all(&data_dir).await;

		let event_bus = broadcast::channel(1024);
		let config = config::Manager::new(data_dir.to_path_buf())
			.await
			.map_err(NodeError::FailedToInitializeConfig)?;

		let (locations, locations_actor) = location::Locations::new();
		// let (old_jobs, jobs_actor) = old_job::OldJobs::new();
		let libraries = library::Libraries::new(data_dir.join("libraries")).await?;

		let (
			get_cloud_api_address,
			cloud_p2p_relay_url,
			cloud_p2p_dns_origin_name,
			cloud_p2p_dns_pkarr_url,
			cloud_services_domain_name,
		) = {
			#[cfg(debug_assertions)]
			{
				(
					std::env::var("SD_CLOUD_API_ADDRESS_URL").unwrap_or_else(|_| {
						format!("{AUTH_SERVER_URL}/cloud-api-address").to_string()
					}),
					std::env::var("SD_CLOUD_P2P_RELAY_URL")
						// .unwrap_or_else(|_| "https://use1-1.relay.iroh.network/".to_string()),
						// .unwrap_or_else(|_| "http://localhost:8081/".to_string()),
						.unwrap_or_else(|_| "https://relay.spacedrive.com:4433/".to_string()),
					std::env::var("SD_CLOUD_P2P_DNS_ORIGIN_NAME")
						// .unwrap_or_else(|_| "dns.iroh.link/".to_string()),
						// .unwrap_or_else(|_| "irohdns.localhost".to_string()),
						.unwrap_or_else(|_| "irohdns.spacedrive.com".to_string()),
					std::env::var("SD_CLOUD_P2P_DNS_PKARR_URL")
						// .unwrap_or_else(|_| "https://dns.iroh.link/pkarr".to_string()),
						// .unwrap_or_else(|_| "http://localhost:8080/pkarr".to_string()),
						.unwrap_or_else(|_| "https://irohdns.spacedrive.com/pkarr".to_string()),
					std::env::var("SD_CLOUD_API_DOMAIN_NAME")
						// .unwrap_or_else(|_| "localhost".to_string()),
						.unwrap_or_else(|_| "cloud.spacedrive.com".to_string()),
				)
			}
			#[cfg(not(debug_assertions))]
			{
				(
					"https://auth.spacedrive.com/cloud-api-address".to_string(),
					"https://relay.spacedrive.com/".to_string(),
					"irohdns.spacedrive.com".to_string(),
					"irohdns.spacedrive.com/pkarr".to_string(),
					"api.spacedrive.com".to_string(),
				)
			}
		};

		let task_system = TaskSystem::new();

		// let (p2p, start_p2p) = p2p::P2PManager::new(config.clone(), libraries.clone())
		// 	.await
		// 	.map_err(NodeError::P2PManager)?;

		let device_id = config.get().await.id;
		let volume_ctx = volume::VolumeManagerContext {
			device_id: device_id.clone().into(),
			library_event_tx: libraries.rx.clone(),
		};

		let (volumes, volume_manager_actor) = VolumeManagerActor::new(Arc::new(volume_ctx)).await?;

		let volumes = Arc::new(volumes);

		let node = Arc::new(Node {
			data_dir: data_dir.to_path_buf(),
			job_system: JobSystem::new(task_system.get_dispatcher(), data_dir),
			task_system,
			volumes,
			locations,
			notifications: notifications::Notifications::new(),
			// p2p,
			config,
			event_bus,
			libraries,
			cloud_services: Arc::new(
				CloudServices::new(
					&get_cloud_api_address,
					cloud_p2p_relay_url,
					cloud_p2p_dns_pkarr_url,
					cloud_p2p_dns_origin_name,
					cloud_services_domain_name,
				)
				.await?,
			),
			master_rng: Arc::new(Mutex::new(CryptoRng::new()?)),
			// old_jobs,
		});

		// Setup start actors that depend on the `Node`
		#[cfg(debug_assertions)]
		if let Some(init_data) = init_data {
			init_data.apply(&node.libraries, &node).await?;
		}

		let router = api::mount();

		// Be REALLY careful about ordering here or you'll get unreliable deadlock's!
		locations_actor.start(node.clone());
		node.libraries.init(&node).await?;
		jobs_actor.start(node.clone());
		volume_manager_actor.start(device_id).await;

		node.job_system
			.init(
				&node
					.libraries
					.get_all()
					.await
					.into_iter()
					.map(|library| {
						(
							library.id,
							NodeContext {
								library,
								node: Arc::clone(&node),
							},
						)
					})
					.collect(),
			)
			.await?;

		// Register all job handlers
		node.job_system
			.register_handler::<LocationScanJobRegistration>();

		// start_p2p(
		// 	node.clone(),
		// 	axum::Router::new()
		// 		.nest(
		// 			"/uri",
		// 			custom_uri::base_router().with_state(custom_uri::with_state(node.clone())),
		// 		)
		// 		.nest(
		// 			"/rspc",
		// 			router
		// 				.clone()
		// 				.endpoint({
		// 					let node = node.clone();
		// 					move |_| node.clone()
		// 				})
		// 				.axum::<()>(),
		// 		)
		// 		.into_make_service(),
		// );

		// save_storage_statistics(&node);

		info!("Spacedrive online!");
		Ok((node, router))
	}

	pub fn init_logger(data_dir: impl AsRef<Path>) -> Result<WorkerGuard, FromEnvError> {
		let (logfile, guard) = NonBlocking::new(
			RollingFileAppender::builder()
				.filename_prefix("sd.log")
				.rotation(Rotation::DAILY)
				.max_log_files(4)
				.build(data_dir.as_ref().join("logs"))
				.expect("Error setting up log file!"),
		);

		// Set a default if the user hasn't set an override
		if std::env::var("RUST_LOG") == Err(std::env::VarError::NotPresent) {
			let level = if cfg!(debug_assertions) {
				"debug"
			} else {
				"info"
			};

			std::env::set_var(
				"RUST_LOG",
				format!(
					"info,\
					iroh_net=info,\
					sd_core={level},\
					sd_p2p={level},\
					sd_core_heavy_lifting={level},\
					sd_file_actions={level},\
					sd_task_system={level},\
					sd_ai={level}"
				),
			);
		}

		let registry = registry();

		let registry = registry
			.with(
				tracing_subscriber::fmt::layer()
					.with_file(true)
					.with_line_number(true)
					.with_ansi(false)
					.with_target(true)
					.with_writer(logfile)
					.with_filter(EnvFilter::from_default_env()),
			)
			.with(
				tracing_subscriber::fmt::layer()
					.with_file(true)
					.with_line_number(true)
					.with_writer(std::io::stdout)
					.event_format(Format::default().pretty())
					.with_filter(EnvFilter::from_default_env()),
			);

		#[cfg(target_os = "android")]
		let registry = registry.with(tracing_android::layer("com.spacedrive.app").unwrap());

		registry.init();

		std::panic::set_hook(Box::new(move |panic| {
			use std::backtrace::{Backtrace, BacktraceStatus};
			let backtrace = Backtrace::capture();
			if let Some(location) = panic.location() {
				tracing::error!(
					message = %panic,
					panic.file = format!("{}:{}", location.file(), location.line()),
					panic.column = location.column(),
				);
				if backtrace.status() == BacktraceStatus::Captured {
					// NOTE(matheus-consoli): it seems that `tauri` is messing up the stack-trace
					// and it doesn't capture anything, even when `RUST_BACKTRACE=full`,
					// so in the current architecture, this is emitting an empty event.
					tracing::error!(message = %backtrace);
				}
			} else {
				tracing::error!(message = %panic);
			}
		}));

		Ok(guard)
	}

	pub async fn shutdown(&self) {
		info!("Spacedrive shutting down...");

		// Let's shutdown the task system first, as the job system will receive tasks to save
		self.task_system.shutdown().await;

		(
			self.old_jobs.shutdown(),
			self.p2p.shutdown(),
			self.job_system.shutdown(),
		)
			.join()
			.await;

		info!("Spacedrive Core shutdown successful!");
	}

	pub(crate) fn emit(&self, event: CoreEvent) {
		if let Err(e) = self.event_bus.0.send(event) {
			warn!(?e, "Error sending event to event bus;");
		}
	}

	pub async fn ephemeral_thumbnail_exists(
		&self,
		cas_id: &CasId<'_>,
	) -> Result<bool, FileIOError> {
		let thumb_path =
			ThumbnailKind::Ephemeral.compute_path(self.config.data_directory(), cas_id);

		match fs::metadata(&thumb_path).await {
			Ok(_) => Ok(true),
			Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
			Err(e) => Err(FileIOError::from((thumb_path, e))),
		}
	}

	pub async fn emit_notification(&self, data: NotificationData, expires: Option<DateTime<Utc>>) {
		let notification = Notification {
			id: NotificationId::Node(self.notifications._internal_next_id()),
			data,
			read: false,
			expires,
		};

		match self
			.config
			.write(|cfg| cfg.notifications.push(notification.clone()))
			.await
		{
			Ok(_) => {
				self.notifications._internal_send(notification);
			}
			Err(e) => {
				error!(?e, "Error saving notification to config;");
			}
		}
	}
}
