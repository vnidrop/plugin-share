package plugin.vnidrop.share

import android.app.Activity
import android.content.ClipData
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
class SharedFile {
    lateinit var data: String
    lateinit var name: String
    lateinit var mimeType: String
}

@InvokeArg
class ShareOptions {
    var text: String? = null
    var title: String? = null
    var url: String? = null
    var files: List<SharedFile>? = null
}

@TauriPlugin
class SharePlugin(private val activity: Activity): Plugin(activity) {

    @Command
    fun canShare(invoke: Invoke) {
        // The native share sheet is almost always available on Android.
        // This command primarily serves to confirm the plugin is installed and responsive.
        val result = JSObject()
        result.put("value", true)
        invoke.resolve(result)
    }

    @Command
    fun share(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(ShareOptions::class.java)
            val fileUris = ArrayList<Uri>()
            var determinedMimeType = "text/plain"

            args.files?.let {
                if (it.isNotEmpty()) {
                    for (file in it) {
                        val decodedBytes = Base64.decode(file.data, Base64.DEFAULT)
                        val tempFile = createSafeFile(file.name)
                        FileOutputStream(tempFile).use { outputStream ->
                            outputStream.write(decodedBytes)
                        }

                        val authority = "${activity.packageName}.fileprovider"
                        fileUris.add(
                            FileProvider.getUriForFile(activity, authority, tempFile)
                        )
                    }

                    determinedMimeType = determineMimeType(it)
                }
            }

            val shareIntent = Intent()
            if (fileUris.isNotEmpty()) {
                shareIntent.action = if (fileUris.size > 1) Intent.ACTION_SEND_MULTIPLE else Intent.ACTION_SEND
                if (fileUris.size > 1) {
                    shareIntent.putParcelableArrayListExtra(Intent.EXTRA_STREAM, fileUris)
                } else {
                    shareIntent.putExtra(Intent.EXTRA_STREAM, fileUris[0])
                }

                shareIntent.clipData = ClipData.newUri(activity.contentResolver, "Shared Files", fileUris[0])
            } else {
                shareIntent.action = Intent.ACTION_SEND
            }

            shareIntent.type = determinedMimeType

            val combinedText = args.url ?: args.text
            if (combinedText != null) {
                shareIntent.putExtra(Intent.EXTRA_TEXT, combinedText)
            }
            if (args.title != null) {
                shareIntent.putExtra(Intent.EXTRA_TITLE, args.title)
            }

            shareIntent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            val chooser = Intent.createChooser(shareIntent, args.title)
            activity.startActivity(chooser)

            invoke.resolve()
        } catch (e: Exception) {
            invoke.reject("Failed to share content: ${e.message}", e)
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

    private fun determineMimeType(files: List<SharedFile>): String {
        if (files.isEmpty()) return "*/*"
        val firstMimeType = files.first().mimeType
        val firstGeneralType = firstMimeType.substringBefore('/')
        
        val allSame = files.all { it.mimeType == firstMimeType }
        if (allSame) return firstMimeType

        val allSameGeneral = files.all { it.mimeType.startsWith(firstGeneralType) }
        if (allSameGeneral) return "$firstGeneralType/*"

        return "*/*"
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
}