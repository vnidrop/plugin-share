import { invoke } from "@tauri-apps/api/core";

/**
 * The data to be shared, mirroring the Web Share API's ShareData dictionary.
 */
export interface ShareData {
  /** An array of File objects to be shared. */
  files?: File[];
  /** The text content to be shared. */
  text?: string;
  /** A title for the content being shared. */
  title?: string;
  /** A URL to be shared. */
  url?: string;
}

/**
 * Checks if the sharing API is available and if the given data can be shared.
 * On mobile, this will almost always resolve to true, but it's good practice
 * for feature detection and platform consistency.
 * @param data The data to test for shareability.
 * @returns A promise that resolves with a boolean indicating if sharing is possible.
 */
export async function canShare(data?: ShareData): Promise<boolean> {
  // On mobile, the native share sheet is always available.
  // We can add more sophisticated checks here if needed in the future.
  // For now, we confirm the plugin is available.
  const result = (await invoke("plugin:vnidrop-share|can_share")) as {
    value: boolean;
  };
  return result.value === true;
}

/**
 * Manually triggers the cleanup of any temporary files created by the plugin.
 *
 * @returns A promise that resolves when the cleanup operation is complete.
 */
export async function cleanup(): Promise<void> {
  await invoke("plugin:vnidrop-share|cleanup");
}

async function fileToBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.readAsDataURL(file);
    reader.onload = () => {
      // The result includes the data URL prefix (e.g., "data:image/png;base64,"),
      // which we will strip off to send only the raw Base64.
      const base64String = (reader.result as string).split(",")[1];
      resolve(base64String);
    };
    reader.onerror = (error) => reject(error);
  });
}

/**
 * Opens the native system share dialog to share content.
 *
 * @param data The content to share, including text, URLs, and/or files.
 * @returns A promise that resolves when the share dialog is closed.
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
