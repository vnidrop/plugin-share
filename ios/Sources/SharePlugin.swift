import Tauri
import UIKit
import WebKit

// For the 'shareText' command
struct ShareTextOptions: Decodable {
    let text: String
    var title: String?
}

// For the 'shareData' command (Base64 data)
struct ShareDataOptions: Decodable {
    let data: String
    let name: String
    var title: String?
}

// For the 'shareFile' command (file path)
struct ShareFileOptions: Decodable {
    let path: String
    var title: String?
}

@_cdecl("init_plugin_share") 
func initPlugin() -> Plugin {
    return SharePlugin()
}

public class SharePlugin: Plugin {

    /**
     * Opens the Sharesheet to share plain text.
     */
    @objc func shareText(_ invoke: Invoke) throws {
        let args = try invoke.parseArgs(ShareTextOptions.self)
        presentShareSheet(invoke: invoke, activityItems: [args.text], title: args.title)
    }

    /**
     * Shares a file whose content is provided as a Base64 string.
     * This method creates a temporary file that is securely handled and
     * automatically cleaned up after the share operation is complete.
     */
    @objc func shareData(_ invoke: Invoke) throws {
        let args = try invoke.parseArgs(ShareDataOptions.self)

        guard let decodedData = Data(base64Encoded: args.data) else {
            invoke.reject("Invalid Base64 data provided.")
            return
        }

        do {
            // Create a secure temporary file URL in a dedicated directory.
            let tempFileURL = try createSafeTempFile(for: args.name)
            
            // Write the data to the temporary file.
            try decodedData.write(to: tempFileURL, options:.atomic)
            
            // Present the share sheet. The completion handler will delete the temp file.
            presentShareSheet(
              invoke: invoke, 
              activityItems: [tempFileURL], 
              title: args.title,
              cleanup: { [weak self] in
                  self?.cleanupTempFile(at: tempFileURL)
              }
              ) 
        } catch {
            invoke.reject("Failed to create or write temporary file: \(error.localizedDescription)")
        }
    }

    /**
     * Shares an existing file from a given file path.
     * Ideal for large files already on the filesystem.
     */
    @objc func shareFile(_ invoke: Invoke) throws {
        let args = try invoke.parseArgs(ShareFileOptions.self)
        let fileURL = URL(fileURLWithPath: args.path)

        // Robustness Check: Ensure the file exists before trying to share.
        guard FileManager.default.fileExists(atPath: fileURL.path) else {
            invoke.reject("File does not exist at the provided path: \(args.path)")
            return
        }

        presentShareSheet(invoke: invoke, activityItems:[fileURL], title: args.title)
    }

    /**
     * Deletes all temporary files created by this plugin.
     * This is a manual cleanup utility for the developer.
     */
    @objc func cleanup(_ invoke: Invoke) {
        do {
            let shareDir = try getSafeShareDir()
            if FileManager.default.fileExists(atPath: shareDir.path) {
                try FileManager.default.removeItem(at: shareDir)
            }
            invoke.resolve()
        } catch {
            invoke.reject("Error during cleanup: \(error.localizedDescription)")
        }
    }

    /**
     * A centralized helper to configure and present the UIActivityViewController.
     * It handles iPad popover presentation and the optional cleanup logic.
     */
    private func presentShareSheet(invoke: Invoke, activityItems: [Any], title: String?, cleanup: (() -> Void)? = nil) {
        DispatchQueue.main.async {
            guard let viewController = self.manager.viewController else {
                invoke.reject("Could not find root view controller.")
                return
            }

            let activityViewController = UIActivityViewController(activityItems: activityItems, applicationActivities: nil)
            
            // This is the crucial part for reliability. This block is guaranteed to be called
            // when the share sheet is dismissed, regardless of the outcome.
            activityViewController.completionWithItemsHandler = { (activityType, completed, returnedItems, error) in
                // Perform the cleanup task (like deleting a temp file) if one was provided.
                cleanup?()
                
                if let anError = error {
                    invoke.reject("Sharing failed: \(anError.localizedDescription)")
                } else {
                    // Resolve the invoke. `completed` is true if an action was taken, false if cancelled.
                    invoke.resolve()
                }
            }

            // On iPad, the share sheet must be presented as a popover.
            if let popoverController = activityViewController.popoverPresentationController {
                popoverController.sourceView = viewController.view
                popoverController.sourceRect = CGRect(
                    x: viewController.view.bounds.midX,
                    y: viewController.view.bounds.midY,
                    width: 0,
                    height: 0
                )
                popoverController.permittedArrowDirections =  []
            }

            viewController.present(activityViewController, animated: true, completion: nil)
        }
    }
    
    /**
     * Returns the URL for a dedicated, secure directory for storing temporary share files.
     * Creates it if it doesn't exist.
     */
    private func getSafeShareDir() throws -> URL {
        let tempDir = FileManager.default.temporaryDirectory
        let shareDir = tempDir.appendingPathComponent("share")
        
        if !FileManager.default.fileExists(atPath: shareDir.path) {
            try FileManager.default.createDirectory(
                at: shareDir,
                withIntermediateDirectories: true
            )
        }
        
        return shareDir
    }

    /**
     * Creates a safe temporary file URL within the dedicated share directory.
     * It sanitizes the filename and prepends a UUID to guarantee uniqueness.
     */
    private func createSafeTempFile(for untrustedFileName: String) throws -> URL {
        let safeDir = try getSafeShareDir()
        
        // Sanitize the filename to remove any path components, preventing traversal.
        let sanitizedBaseName = URL(fileURLWithPath: untrustedFileName).lastPathComponent
        
        // Prepending a UUID guarantees uniqueness and prevents name collisions.
        let finalFileName = "\(UUID().uuidString)-\(sanitizedBaseName)"
        
        return safeDir.appendingPathComponent(finalFileName)
    }

    private func cleanupTempFile(at url: URL) {
        DispatchQueue.global(qos: .utility).async {
            try? FileManager.default.removeItem(at: url)
        }
    }
}