import { invoke } from "@tauri-apps/api/core";

/**
 * Defines the options for sharing plain text.
 */
export interface ShareTextOptions {
  /**
   * The text to be shared.
   * This can be a simple string, a URL, or any other text content.
   */
  text: string;
  /**
   * The title for the share dialog.
   * This is an optional hint for the share sheet, primarily used on Android and Windows.
   */
  title?: string;
}

/**
 * Defines the options for sharing a file from Base64 encoded data.
 * This is useful for sharing dynamically generated content without writing it to a permanent file first.
 */
export interface ShareDataOptions {
  /**
   * The file content, encoded as a Base64 string.
   * Do not include the data URL prefix (e.g., 'data:image/png;base64,').
   */
  data: string;
  /**
   * The name of the file, including its extension (e.g., 'document.pdf', 'image.png').
   * This name will be suggested in the share dialog.
   */
  name: string;
  /**
   * The title for the share dialog.
   * This is an optional hint for the share sheet, primarily used on Android and Windows.
   */
  title?: string;
}

/**
 * Defines the options for sharing a file from the local filesystem.
 */
export interface ShareFileOptions {
  /**
   * The absolute path to the file on the local filesystem.
   * On mobile, this should be a content URI if sharing from an external source.
   */
  path: string;
  /**
   * The title for the share dialog.
   * This is an optional hint for the share sheet, primarily used on Android and Windows.
   */
  title?: string;
}

/**
 * Opens the native system share dialog to share plain text.
 *
 * @param options The options for sharing text.
 * @returns A promise that resolves when the share dialog is closed.
 *
 * @example
 * ```typescript
 * import { shareText } from '@vnidrop/tauri-plugin-share';
 *
 * await shareText({
 * title: 'Share this URL',
 * text: '[https://tauri.app](https://tauri.app)'
 * });
 * ```
 */
export async function shareText(options: ShareTextOptions): Promise<void> {
  await invoke("plugin:vnidrop-share|share_text", { options });
}

/**
 * Opens the native system share dialog to share a file from Base64 data.
 * The plugin handles creating a temporary file and cleaning it up automatically.
 *
 * @param options The options for sharing data.
 * @returns A promise that resolves when the share dialog is closed.
 *
 * @example
 * ```typescript
 * import { shareData } from '@vnidrop/tauri-plugin-share';
 *
 * // Example: Sharing a simple text file created from a Base64 string.
 * const base64Data = btoa('Hello from Tauri!'); // "SGVsbG8gZnJvbSBUYXVyaSE="
 *
 * await shareData({
 * title: 'Share my file',
 * name: 'greeting.txt',
 * data: base64Data
 * });
 * ```
 */
export async function shareData(options: ShareDataOptions): Promise<void> {
  await invoke("plugin:vnidrop-share|share_data", { options });
}

/**
 * Opens the native system share dialog to share a file from the local filesystem.
 *
 * @param options The options for sharing a file.
 * @returns A promise that resolves when the share dialog is closed.
 *
 * @example
 * ```typescript
 * import { shareFile } from '@vnidrop/tauri-plugin-share';
 * import { join } from '@tauri-apps/api/path';
 * import { appDataDir } from '@tauri-apps/api/path';
 *
 * // Note: You must have permissions to access this file path.
 * const filePath = await join(await appDataDir(), 'my-app-file.log');
 *
 * await shareFile({
 * title: 'Share App Log',
 * path: filePath
 * });
 * ```
 */
export async function shareFile(options: ShareFileOptions): Promise<void> {
  await invoke("plugin:vnidrop-share|share_file", { options });
}

/**
 * Manually triggers the cleanup of any temporary files created by the plugin.
 *
 * While the plugin attempts to clean up automatically, this function can be called
 * as a failsafe, for example, on application startup.
 *
 * @returns A promise that resolves when the cleanup operation is complete.
 *
 * @example
 * ```typescript
 * import { cleanup } from '@vnidrop/tauri-plugin-share';
 *
 * // Call on app startup or when you want to ensure no temp files are left.
 * await cleanup();
 * ```
 */
export async function cleanup(): Promise<void> {
  await invoke("plugin:vnidrop-share|cleanup");
}
