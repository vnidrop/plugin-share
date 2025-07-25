package plugin.vnidrop.share

import android.app.Activity
import android.content.ContentResolver
import android.content.Intent
import android.net.Uri
import android.util.Base64
import java.io.File
import java.io.FileOutputStream
import android.provider.OpenableColumns
import androidx.core.content.FileProvider
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin
import java.io.IOException
import java.util.UUID

@InvokeArg
class ShareTextOptions {
    lateinit var text: String
    var mimeType: String = "text/plain"
    var title: String? = null
}

@InvokeArg
class ShareFileByDataOptions {
    lateinit var data: String
    lateinit var name: String
    var mimeType: String = "application/octet-stream"
    var title: String? = null
}

@InvokeArg
class ShareFileByContentUriOptions {
    lateinit var uri: String // The content:// URI string
    var title: String? = null
}

@TauriPlugin
class SharePlugin(private val activity: Activity): Plugin(activity) {
    /**
     * Opens the Sharesheet to share plain text.
     */
    @Command
    fun shareText(invoke: Invoke) {        
        val args = invoke.parseArgs(ShareTextOptions::class.java)

        val sendIntent = Intent().apply {
            action = Intent.ACTION_SEND
            type = args.mimeType
            putExtra(Intent.EXTRA_TEXT, args.text)
            putExtra(Intent.EXTRA_TITLE, args.title)
        }

        val chooser = Intent.createChooser(sendIntent, args.title)

        // Robustness check: Ensure there is an app that can handle this intent.
        if (chooser.resolveActivity(activity.packageManager)!= null) {
            activity.startActivity(chooser)
            invoke.resolve()
        } else {
            invoke.reject("No app found to handle sharing text.")
        }
    }

    /**
     * Shares a file whose content is provided as a Base64 string.
     * Ideal for small, dynamically generated files.
     */
    @Command
    fun shareData(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(ShareFileByDataOptions::class.java)
            val decodedBytes = Base64.decode(args.data, Base64.DEFAULT)

            // Create a secure file in our dedicated share cache
            val tempFile = createSafeFile(args.name)

            // Write the decoded data to the secure file
            FileOutputStream(tempFile).use { it.write(decodedBytes) }

            // Launch the share intent
            launchShareIntent(invoke, tempFile, args.mimeType, args.title)
        } catch (e: Exception) {
            invoke.reject("Failed to share file from data: ${e.message}", e)
        }
    }

    /**
     * Securely shares a file from an external source (e.g., file picker) using its content URI.
     * This method follows Android's best practices by copying the file content into the app's
     * private cache, ensuring the app has proper authority to share it via FileProvider.
     */
    @Command
    fun shareFile(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(ShareFileByContentUriOptions::class.java)
            val contentUri = Uri.parse(args.uri)
            val contentResolver = activity.contentResolver

            val fileName = getFileNameFromUri(contentResolver, contentUri)

            val mimeType = contentResolver.getType(contentUri)?: "application/octet-stream"

            val tempFile = createSafeFile(fileName)

            contentResolver.openInputStream(contentUri)?.use { inputStream ->
                FileOutputStream(tempFile).use { outputStream ->
                    inputStream.copyTo(outputStream)
                }
            }?: throw IOException("Failed to open input stream for URI: $contentUri")

            launchShareIntent(invoke, tempFile, mimeType, args.title)

        } catch (e: Exception) {
            invoke.reject("Failed to share file from URI: ${e.message}", e)
        }
    }

    /**
     * Deletes all temporary files created by this plugin in its dedicated share directory.
     * This should be called by the developer when the files are no longer needed.
     */
    @Command
    fun cleanup(invoke: Invoke) {
        try {
            val shareDir = getSafeShareDir()
            if (shareDir.exists() && shareDir.isDirectory) {
                if (!shareDir.deleteRecursively()) {
                    invoke.reject("Failed to delete all temporary share files.")
                    return
                }
            }
            invoke.resolve()
        } catch (e: Exception) {
            invoke.reject("Error during cleanup: ${e.message}", e)
        }
    }

    /**
     * Creates and launches the ACTION_SEND Intent using the configured FileProvider.
     */
    private fun launchShareIntent(invoke: Invoke, file: File, mimeType: String, title: String?) {
        val authority = "${activity.packageName}.fileprovider"
        val contentUri = FileProvider.getUriForFile(activity, authority, file)

        val shareIntent = Intent(Intent.ACTION_SEND).apply {
            type = mimeType
            putExtra(Intent.EXTRA_STREAM, contentUri)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            if (title!= null) {
                putExtra(Intent.EXTRA_TITLE, title)
            }
        }

        val chooser = Intent.createChooser(shareIntent, title)
        activity.startActivity(chooser)
        invoke.resolve()
    }

    /**
     * Returns the dedicated, secure directory for storing temporary share files.
     * Creates it if it doesn't exist.
     */
    private fun getSafeShareDir(): File {
        val shareDir = File(activity.cacheDir, "shares")
        if (!shareDir.exists()) {
            shareDir.mkdirs()
        }
        return shareDir
    }

    /**
     * Creates a safe File object within the dedicated share directory.
     * It sanitizes the filename and performs path traversal checks.
     */
    @Throws(IOException::class, SecurityException::class)
    private fun createSafeFile(untrustedFileName: String): File {
        val safeDir = getSafeShareDir()
        val safeDirCanonicalPath = safeDir.canonicalPath

        // Sanitize the filename to prevent malicious characters.
        // A robust approach is to allow only a whitelist of characters.
        // Here, we also add a UUID to prevent name collisions.
        val sanitizedBaseName = untrustedFileName.replace(Regex("[^a-zA-Z0-9._-]"), "")
        val finalFileName = "${UUID.randomUUID()}-${sanitizedBaseName}"

        if (finalFileName.isEmpty()) {
            throw SecurityException("Invalid filename: sanitized name is empty.")
        }

        val intendedFile = File(safeDir, finalFileName)

        // CRITICAL: Path Traversal Check
        // Ensure the final resolved path is still inside our secure directory.
        if (!intendedFile.canonicalPath.startsWith(safeDirCanonicalPath + File.separator)) {
            throw SecurityException("Path Traversal Attack Detected. Malicious filename: '$untrustedFileName'")
        }

        return intendedFile
    }

    /**
     * Safely retrieves the display name of a file from a content URI.
     */
    private fun getFileNameFromUri(resolver: ContentResolver, uri: Uri): String {
        var fileName = "unknown_file"
        resolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
            if (cursor.moveToFirst()) {
                val nameIndex = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                if (nameIndex!= -1) {
                    fileName = cursor.getString(nameIndex)
                }
            }
        }
        return fileName
    }
}