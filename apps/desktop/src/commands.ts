/** tauri-specta globals **/

import { Channel as TAURI_CHANNEL, invoke as TAURI_INVOKE } from '@tauri-apps/api/core';
import * as TAURI_API_EVENT from '@tauri-apps/api/event';
import { type WebviewWindow as __WebviewWindow__ } from '@tauri-apps/api/webviewWindow';

/* eslint-disable */
// This file was generated by [tauri-specta](https://github.com/oscartbeaumont/tauri-specta). Do not edit this file manually.

/** user-defined commands **/

export const commands = {
	async appReady(): Promise<void> {
		await TAURI_INVOKE('app_ready');
	},
	async resetSpacedrive(): Promise<void> {
		await TAURI_INVOKE('reset_spacedrive');
	},
	async openLogsDir(): Promise<Result<null, null>> {
		try {
			return { status: 'ok', data: await TAURI_INVOKE('open_logs_dir') };
		} catch (e) {
			if (e instanceof Error) throw e;
			else return { status: 'error', error: e as any };
		}
	},
	async refreshMenuBar(): Promise<Result<null, null>> {
		try {
			return { status: 'ok', data: await TAURI_INVOKE('refresh_menu_bar') };
		} catch (e) {
			if (e instanceof Error) throw e;
			else return { status: 'error', error: e as any };
		}
	},
	async reloadWebview(): Promise<void> {
		await TAURI_INVOKE('reload_webview');
	},
	async setMenuBarItemState(event: MenuEvent, enabled: boolean): Promise<void> {
		await TAURI_INVOKE('set_menu_bar_item_state', { event, enabled });
	},
	async requestFdaMacos(): Promise<void> {
		await TAURI_INVOKE('request_fda_macos');
	},
	async openTrashInOsExplorer(): Promise<Result<null, null>> {
		try {
			return { status: 'ok', data: await TAURI_INVOKE('open_trash_in_os_explorer') };
		} catch (e) {
			if (e instanceof Error) throw e;
			else return { status: 'error', error: e as any };
		}
	},
	/**
	 * Initiates a drag and drop operation with cursor position tracking
	 *
	 * # Arguments
	 * * `window` - The Tauri window instance
	 * * `_state` - Current drag state (unused)
	 * * `files` - Vector of file paths to be dragged
	 * * `icon_path` - Path to the preview icon for the drag operation
	 * * `on_event` - Channel for communicating drag operation events back to the frontend
	 */
	async startDrag(
		files: string[],
		iconPath: string,
		onEvent: TAURI_CHANNEL<CallbackResult>
	): Promise<Result<null, string>> {
		try {
			return {
				status: 'ok',
				data: await TAURI_INVOKE('start_drag', { files, iconPath, onEvent })
			};
		} catch (e) {
			if (e instanceof Error) throw e;
			else return { status: 'error', error: e as any };
		}
	},
	async stopDrag(): Promise<void> {
		await TAURI_INVOKE('stop_drag');
	},
	async openFilePaths(
		library: string,
		ids: number[]
	): Promise<Result<OpenFilePathResult[], null>> {
		try {
			return { status: 'ok', data: await TAURI_INVOKE('open_file_paths', { library, ids }) };
		} catch (e) {
			if (e instanceof Error) throw e;
			else return { status: 'error', error: e as any };
		}
	},
	async openEphemeralFiles(paths: string[]): Promise<Result<EphemeralFileOpenResult[], null>> {
		try {
			return { status: 'ok', data: await TAURI_INVOKE('open_ephemeral_files', { paths }) };
		} catch (e) {
			if (e instanceof Error) throw e;
			else return { status: 'error', error: e as any };
		}
	},
	async getFilePathOpenWithApps(
		library: string,
		ids: number[]
	): Promise<Result<OpenWithApplication[], null>> {
		try {
			return {
				status: 'ok',
				data: await TAURI_INVOKE('get_file_path_open_with_apps', { library, ids })
			};
		} catch (e) {
			if (e instanceof Error) throw e;
			else return { status: 'error', error: e as any };
		}
	},
	async getEphemeralFilesOpenWithApps(
		paths: string[]
	): Promise<Result<OpenWithApplication[], null>> {
		try {
			return {
				status: 'ok',
				data: await TAURI_INVOKE('get_ephemeral_files_open_with_apps', { paths })
			};
		} catch (e) {
			if (e instanceof Error) throw e;
			else return { status: 'error', error: e as any };
		}
	},
	async openFilePathWith(
		library: string,
		fileIdsAndUrls: [number, string][]
	): Promise<Result<null, null>> {
		try {
			return {
				status: 'ok',
				data: await TAURI_INVOKE('open_file_path_with', { library, fileIdsAndUrls })
			};
		} catch (e) {
			if (e instanceof Error) throw e;
			else return { status: 'error', error: e as any };
		}
	},
	async openEphemeralFileWith(pathsAndUrls: [string, string][]): Promise<Result<null, null>> {
		try {
			return {
				status: 'ok',
				data: await TAURI_INVOKE('open_ephemeral_file_with', { pathsAndUrls })
			};
		} catch (e) {
			if (e instanceof Error) throw e;
			else return { status: 'error', error: e as any };
		}
	},
	async revealItems(library: string, items: RevealItem[]): Promise<Result<null, null>> {
		try {
			return { status: 'ok', data: await TAURI_INVOKE('reveal_items', { library, items }) };
		} catch (e) {
			if (e instanceof Error) throw e;
			else return { status: 'error', error: e as any };
		}
	},
	async lockAppTheme(themeType: AppThemeType): Promise<void> {
		await TAURI_INVOKE('lock_app_theme', { themeType });
	},
	async checkForUpdate(): Promise<Result<Update | null, string>> {
		try {
			return { status: 'ok', data: await TAURI_INVOKE('check_for_update') };
		} catch (e) {
			if (e instanceof Error) throw e;
			else return { status: 'error', error: e as any };
		}
	},
	async installUpdate(): Promise<Result<null, string>> {
		try {
			return { status: 'ok', data: await TAURI_INVOKE('install_update') };
		} catch (e) {
			if (e instanceof Error) throw e;
			else return { status: 'error', error: e as any };
		}
	}
};

/** user-defined events **/

export const events = __makeEvents__<{
	dragAndDropEvent: DragAndDropEvent;
}>({
	dragAndDropEvent: 'drag-and-drop-event'
});

/** user-defined constants **/

/** user-defined types **/

export type AppThemeType = 'Auto' | 'Light' | 'Dark';
export type CallbackResult = { result: WrappedDragResult; cursorPos: WrappedCursorPosition };
export type DragAndDropEvent =
	| { type: 'Hovered'; paths: string[]; x: number; y: number }
	| { type: 'Dropped'; paths: string[]; x: number; y: number }
	| { type: 'Cancelled' };
export type EphemeralFileOpenResult = { t: 'Ok'; c: string } | { t: 'Err'; c: string };
export type MenuEvent =
	| 'NewLibrary'
	| 'NewFile'
	| 'NewDirectory'
	| 'AddLocation'
	| 'OpenOverview'
	| 'OpenSearch'
	| 'OpenSettings'
	| 'ReloadExplorer'
	| 'SetLayoutGrid'
	| 'SetLayoutList'
	| 'SetLayoutMedia'
	| 'ToggleDeveloperTools'
	| 'NewWindow'
	| 'ReloadWebview'
	| 'Copy'
	| 'Cut'
	| 'Paste'
	| 'Duplicate'
	| 'SelectAll';
export type OpenFilePathResult =
	| { t: 'NoLibrary' }
	| { t: 'NoFile'; c: number }
	| { t: 'OpenError'; c: [number, string] }
	| { t: 'AllGood'; c: number }
	| { t: 'Internal'; c: string };
export type OpenWithApplication = { url: string; name: string };
export type RevealItem =
	| { Location: { id: number } }
	| { FilePath: { id: number } }
	| { Ephemeral: { path: string } };
export type Update = { version: string };
export type WrappedCursorPosition = { x: number; y: number };
export type WrappedDragResult = 'Dropped' | 'Cancel';

type __EventObj__<T> = {
	listen: (cb: TAURI_API_EVENT.EventCallback<T>) => ReturnType<typeof TAURI_API_EVENT.listen<T>>;
	once: (cb: TAURI_API_EVENT.EventCallback<T>) => ReturnType<typeof TAURI_API_EVENT.once<T>>;
	emit: null extends T
		? (payload?: T) => ReturnType<typeof TAURI_API_EVENT.emit>
		: (payload: T) => ReturnType<typeof TAURI_API_EVENT.emit>;
};

export type Result<T, E> = { status: 'ok'; data: T } | { status: 'error'; error: E };

function __makeEvents__<T extends Record<string, any>>(mappings: Record<keyof T, string>) {
	return new Proxy(
		{} as unknown as {
			[K in keyof T]: __EventObj__<T[K]> & {
				(handle: __WebviewWindow__): __EventObj__<T[K]>;
			};
		},
		{
			get: (_, event) => {
				const name = mappings[event as keyof T];

				return new Proxy((() => {}) as any, {
					apply: (_, __, [window]: [__WebviewWindow__]) => ({
						listen: (arg: any) => window.listen(name, arg),
						once: (arg: any) => window.once(name, arg),
						emit: (arg: any) => window.emit(name, arg)
					}),
					get: (_, command: keyof __EventObj__<any>) => {
						switch (command) {
							case 'listen':
								return (arg: any) => TAURI_API_EVENT.listen(name, arg);
							case 'once':
								return (arg: any) => TAURI_API_EVENT.once(name, arg);
							case 'emit':
								return (arg: any) => TAURI_API_EVENT.emit(name, arg);
						}
					}
				});
			}
		}
	);
}
