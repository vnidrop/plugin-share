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
}