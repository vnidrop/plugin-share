import { invoke } from "@tauri-apps/api/core";

/**
 * Represents the content to be shared, similar to the Web Share API's ShareData dictionary.
 *
 * Example:
 * ```ts
 * const shareData: ShareData = {
 *   title: "Check this out!",
 *   text: "Here's an interesting article.",
 *   url: "https://example.com/article",
 *   files: [myFile]
 * };
 * ```
 */
export interface ShareData {
  /** Optional array of File objects to share (e.g., images, PDFs). */
  files?: File[];
  /** Optional text content to be shared. */
  text?: string;
  /** Optional title describing the shared content. */
  title?: string;
  /** Optional URL to be shared. */
  url?: string;
}

/**
 * Checks whether the native sharing capability is available for the given data.
 *
 * On mobile platforms, this will typically return `true`.
 * This is useful for feature detection before attempting to share.
 *
 * Example:
 * ```ts
 * if (await canShare({ text: "Hello World" })) {
 *   console.log("Sharing is supported!");
 * } else {
 *   console.log("Sharing is not available on this platform.");
 * }
 * ```
 *
 * @param data Optional ShareData to check shareability for.
 * @returns Promise resolving to `true` if sharing is possible.
 */
export async function canShare(data?: ShareData): Promise<boolean> {
  const result = (await invoke("plugin:vnidrop-share|can_share")) as {
    value: any;
  };
  return result.value === true || result.value === "true";
}

/**
 * Manually triggers cleanup of temporary files created by the plugin.
 *
 * Useful when files are generated during sharing but you want to remove them
 * immediately after to save storage space.
 *
 * Example:
 * ```ts
 * await cleanup();
 * console.log("Temporary share files removed.");
 * ```
 *
 * @returns Promise resolving when cleanup is complete.
 */
export async function cleanup(): Promise<void> {
  await invoke("plugin:vnidrop-share|cleanup");
}

/**
 * Converts a `File` object to a Base64-encoded string (without the Data URL prefix).
 *
 * Example:
 * ```ts
 * const base64Data = await fileToBase64(myFile);
 * console.log(base64Data.slice(0, 50)); // preview first 50 chars
 * ```
 *
 * @param file File to convert.
 * @returns Promise resolving to Base64 string.
 */
async function fileToBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.readAsDataURL(file);
    reader.onload = () => {
      const base64String = (reader.result as string).split(",")[1];
      resolve(base64String);
    };
    reader.onerror = (error) => reject(error);
  });
}

/**
 * Opens the native share dialog to share text, URLs, and/or files.
 *
 * Example:
 * ```ts
 * await share({
 *   title: "My Photo",
 *   text: "Check out this picture!",
 *   files: [myImageFile]
 * });
 * console.log("Share dialog closed.");
 * ```
 *
 * @param data Content to share.
 * @returns Promise resolving when the share dialog is closed.
 */
export async function share(data: ShareData): Promise<void> {
  const payload: any = {
    text: data.text,
    title: data.title,
    url: data.url,
  };

  if (data.files && data.files.length > 0) {
    payload.files = await Promise.all(
      data.files.map(async (file) => ({
        data: await fileToBase64(file),
        name: file.name,
        mimeType: file.type || "application/octet-stream",
      }))
    );
  }

  await invoke("plugin:vnidrop-share|share", { options: payload });
}
