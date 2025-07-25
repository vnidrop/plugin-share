// Source : https://github.com/rittme/tauri-plugin-sharesheet/blob/main/android/src/main/java/SharesheetPlugin.kt

package plugin.vnidrop.share

import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.webkit.WebView
import android.net.Uri
import android.util.Base64
import java.io.File
import java.io.FileOutputStream
import android.content.Context
import androidx.core.content.FileProvider
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin
import java.io.IOException
import java.net.URLDecoder
import java.util.UUID

@InvokeArg
class ShareTextOptions {
    lateinit var text: String
    var mimeType: String = "text/plain"
    var title: String? = null
}

@InvokeArg
class ShareFileOptions {
    lateinit var data: String
    lateinit var name: String
    var mimeType: String = "application/octet-stream"
    var title: String? = null
}

@TauriPlugin
class SharePlugin(private val activity: Activity): Plugin(activity) {
    /**
     * Open the Sharesheet to share some text
     */
    @Command
    fun shareText(invoke: Invoke) {        
        val args = invoke.parseArgs(ShareTextOptions::class.java)

        val sendIntent = Intent().apply {
            this.action = Intent.ACTION_SEND
            this.type = args.mimeType
            this.putExtra(Intent.EXTRA_TEXT, args.text)
            this.putExtra(Intent.EXTRA_TITLE, args.title)
        }

        val shareIntent = Intent.createChooser(sendIntent, null)
        shareIntent.setFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        activity.applicationContext?.startActivity(shareIntent)
    }
    
    /**
     * Open the Sharesheet to share a file
     */
    @Command
    fun shareFile(invoke: Invoke) {
        val args = invoke.parseArgs(ShareFileOptions::class.java)
        
        try {
            // Decode the base64 string to bytes
            val decodedBytes = Base64.decode(args.data, Base64.DEFAULT)
            
            // Create a temporary file to store the data
            val tempFile = File(activity.cacheDir, args.name)
            val outputStream = FileOutputStream(tempFile)
            outputStream.write(decodedBytes)
            outputStream.close()
            
            // Get the authority from the app's manifest
            val authority = "${activity.packageName}.fileprovider"
            
            // Create a content URI for the file
            val contentUri = FileProvider.getUriForFile(activity, authority, tempFile)
            
            // Create and start the share intent
            val shareIntent = Intent(Intent.ACTION_SEND).apply {
                type = args.mimeType
                putExtra(Intent.EXTRA_STREAM, contentUri)
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                if (args.title != null) {
                    putExtra(Intent.EXTRA_TITLE, args.title)
                }
            }
            
            val chooserIntent = Intent.createChooser(shareIntent, null)
            chooserIntent.setFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            activity.applicationContext?.startActivity(chooserIntent)
            
        } catch (e: Exception) {
            invoke.reject("Failed to share file: ${e.message}", e)
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
    private fun createSafeFileForData(untrustedFileName: String): File {
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
     * Parses a Tauri asset protocol URL, decodes the file path, and validates it
     * against path traversal vulnerabilities.
     */
    @Throws(IOException::class, SecurityException::class)
    private fun getValidatedSourceFileFromAssetUrl(assetUrl: String): File {
        if (!assetUrl.startsWith("asset://localhost/")) {
            throw IllegalArgumentException("Invalid asset URL. Must start with 'asset://localhost/'.")
        }

        val encodedPath = assetUrl.substring("asset://localhost/".length)
        val untrustedPath = URLDecoder.decode(encodedPath, "UTF-8")

        val sourceFile = File(untrustedPath)
        val canonicalPath = sourceFile.canonicalPath

        // 4. CRITICAL: Validate the canonical path against the app's data directories.
        // This check ensures the path is within a legitimate app folder, preventing
        // access to arbitrary system files like /etc/passwd.
        // This should align with your `assetScope` in `tauri.conf.json`.
        val cacheDir = activity.cacheDir.canonicalPath
        val filesDir = activity.filesDir.canonicalPath

        if (!canonicalPath.startsWith(cacheDir) &&!canonicalPath.startsWith(filesDir)) {
            throw SecurityException("Path Traversal Attack Detected. Asset path '$untrustedPath' is outside the allowed scope.")
        }

        if (!sourceFile.exists() ||!sourceFile.isFile) {
            throw IOException("Source file does not exist or is not a file: $untrustedPath")
        }

        return sourceFile
    }
}