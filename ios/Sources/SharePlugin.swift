import Tauri
import UIKit
import WebKit

struct SharedFile: Decodable {
    let data: String
    let name: String
    let mimeType: String
}

struct ShareOptions: Decodable {
    var text: String?
    var title: String?
    var url: String?
    var files: [SharedFile]?
}

@_cdecl("init_plugin_share") 
func initPlugin() -> Plugin {
    return SharePlugin()
}

public class SharePlugin: Plugin {

    private var temporaryFileURLs: [URL] = []

    @objc func canShare(_ invoke: Invoke) throws {
        // The native share sheet is always available on iOS.
        invoke.resolve(["value": true])
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

    @objc func share(_ invoke: Invoke) throws {
        let args = try invoke.parseArgs(ShareOptions.self)
        var activityItems: [Any] = []

        if let urlString = args.url, let url = URL(string: urlString) {
            activityItems.append(url)
        }
        if let text = args.text {
            activityItems.append(text)
        }

        if let files = args.files {
            for file in files {
                guard let decodedData = Data(base64Encoded: file.data) else {
                    invoke.reject("Invalid Base64 data for file: \(file.name)")
                    return
                }
                
                do {
                    let tempFileURL = try createSafeTempFile(for: file.name)
                    try decodedData.write(to: tempFileURL, options:.atomic)
                    activityItems.append(tempFileURL)
                    temporaryFileURLs.append(tempFileURL)
                } catch {
                    invoke.reject("Failed to create temporary file: \(error.localizedDescription)")
                    return
                }
            }
        }

        if activityItems.isEmpty {
            invoke.reject("No content provided to share.")
            return
        }

        presentShareSheet(invoke: invoke, activityItems: activityItems)
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
        
        let sanitizedBaseName = URL(fileURLWithPath: untrustedFileName).lastPathComponent
        
        let finalFileName = "\(UUID().uuidString)-\(sanitizedBaseName)"
        
        return safeDir.appendingPathComponent(finalFileName)
    }

    private func presentShareSheet(invoke: Invoke, activityItems: [Any]) {
        DispatchQueue.main.async {
            guard let viewController = self.manager.viewController else {
                invoke.reject("Could not find root view controller.")
                return
            }

            let activityViewController = UIActivityViewController(activityItems: activityItems, applicationActivities: nil)
            
            activityViewController.completionWithItemsHandler = { _, _, _, error in
                self.cleanupTemporaryFiles()
                
                if let anError = error {
                    invoke.reject("Sharing failed: \(anError.localizedDescription)")
                } else {
                    invoke.resolve()
                }
            }

            // iPad presentation logic
            if let popoverController = activityViewController.popoverPresentationController {
                popoverController.sourceView = viewController.view
                popoverController.sourceRect = CGRect(x: viewController.view.bounds.midX, y: viewController.view.bounds.midY, width: 0, height: 0)
                popoverController.permittedArrowDirections = []
            }

            viewController.present(activityViewController, animated: true, completion: nil)
        }
    }

    private func cleanupTemporaryFiles() {
        DispatchQueue.global(qos:.utility).async {
            for url in self.temporaryFileURLs {
                try? FileManager.default.removeItem(at: url)
            }
            self.temporaryFileURLs.removeAll()
        }
    }
}