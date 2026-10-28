import Foundation
import ImageIO
import CoreGraphics
import AVFoundation
import UniformTypeIdentifiers

struct Request: Decodable {
    var operation: String
    var path: String
    var output: String?
    var maxSize: Int?
}

func writeJPEG(_ image: CGImage, to path: String) throws {
    guard let space = CGColorSpace(name: CGColorSpace.sRGB),
          let ctx = CGContext(data: nil, width: image.width, height: image.height, bitsPerComponent: 8,
                              bytesPerRow: 0, space: space, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else {
        throw NSError(domain: "color_conversion", code: 1)
    }
    ctx.setFillColor(CGColor(gray: 0, alpha: 1)); ctx.fill(CGRect(x: 0, y: 0, width: image.width, height: image.height))
    ctx.draw(image, in: CGRect(x: 0, y: 0, width: image.width, height: image.height))
    guard let converted = ctx.makeImage(),
          let dest = CGImageDestinationCreateWithURL(URL(fileURLWithPath: path) as CFURL, UTType.jpeg.identifier as CFString, 1, nil) else {
        throw NSError(domain: "thumbnail_write", code: 1)
    }
    // Only encoder options are supplied: source EXIF/GPS is never copied into thumbnails.
    CGImageDestinationAddImage(dest, converted, [kCGImageDestinationLossyCompressionQuality: 0.78] as CFDictionary)
    guard CGImageDestinationFinalize(dest) else { throw NSError(domain: "thumbnail_write", code: 2) }
}

@main struct Worker {
    static func main() async {
        do {
            let data = FileHandle.standardInput.readDataToEndOfFile()
            guard data.count <= 1_048_576 else { throw NSError(domain: "request_limit", code: 1) }
            let req = try JSONDecoder().decode(Request.self, from: data)
            guard let raw = Data(base64Encoded: req.path), !raw.contains(0) else { throw NSError(domain: "path", code: 1) }
            let url = (raw + Data([0])).withUnsafeBytes {
                URL(fileURLWithFileSystemRepresentation: $0.baseAddress!.assumingMemoryBound(to: CChar.self), isDirectory: false, relativeTo: nil)
            }
            var result: [String: Any] = [:]
            if req.operation == "source" {
                let values = try url.resourceValues(forKeys: [.volumeUUIDStringKey, .volumeURLKey, .volumeIsLocalKey, .isDirectoryKey, .isSymbolicLinkKey])
                guard values.isDirectory == true, values.isSymbolicLink != true, values.volumeIsLocal == true else {
                    throw NSError(domain: "local_folder_required", code: 1)
                }
                result["volume_id"] = values.volumeUUIDString ?? ""
                result["mount"] = values.volume?.path ?? "/"
                let volumeValues = try url.resourceValues(forKeys: [.volumeLocalizedFormatDescriptionKey, .volumeIsInternalKey, .volumeIsReadOnlyKey])
                result["filesystem"] = volumeValues.volumeLocalizedFormatDescription ?? ""
                result["drive_kind"] = volumeValues.volumeIsInternal == true ? "internal" : "external"
                result["read_only"] = volumeValues.volumeIsReadOnly
            } else {
                let values = try url.resourceValues(forKeys: [.isUbiquitousItemKey, .ubiquitousItemDownloadingStatusKey])
                if values.isUbiquitousItem == true && values.ubiquitousItemDownloadingStatus != .current && values.ubiquitousItemDownloadingStatus != .downloaded {
                    throw NSError(domain: "not_downloaded", code: 1)
                }
                let maxSize = min(max(req.maxSize ?? 512, 64), 2560)
                let ext = url.pathExtension.lowercased()
                if ["mov", "mp4", "m4v", "3gp"].contains(ext) {
                    let asset = AVURLAsset(url: url)
                    let duration = try await asset.load(.duration)
                    if duration.seconds.isFinite { result["duration"] = duration.seconds }
                    let tracks = try await asset.loadTracks(withMediaType: .video)
                    if let track = tracks.first {
                        let size = try await track.load(.naturalSize)
                        let transform = try await track.load(.preferredTransform)
                        let transformed = size.applying(transform)
                        result["width"] = Int(abs(transformed.width)); result["height"] = Int(abs(transformed.height))
                    }
                    for item in try await asset.load(.commonMetadata) {
                        if item.commonKey == .commonKeyCreationDate, let text = try? await item.load(.stringValue) { result["captured"] = text }
                        if item.commonKey == .commonKeyModel, let text = try? await item.load(.stringValue) { result["camera"] = String(text.prefix(256)) }
                        if item.commonKey == .commonKeyLocation, let text = try? await item.load(.stringValue) { result["location_raw"] = String(text.prefix(256)) }
                    }
                    result["metadata_state"] = "ready"
                    if let output = req.output {
                        do {
                            let generator = AVAssetImageGenerator(asset: asset)
                            generator.appliesPreferredTrackTransform = true
                            generator.maximumSize = CGSize(width: maxSize, height: maxSize)
                            let seconds = duration.seconds.isFinite ? min(1, max(0, duration.seconds / 10)) : 0
                            let image = try await generator.image(at: CMTime(seconds: seconds, preferredTimescale: 600)).image
                            try writeJPEG(image, to: output)
                            result["thumbnail_state"] = "ready"
                        } catch { result["thumbnail_state"] = "failed"; result["error"] = "video_thumbnail_failed" }
                    }
                } else {
                    guard let source = CGImageSourceCreateWithURL(url as CFURL, [kCGImageSourceShouldCache: false] as CFDictionary),
                          let props = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [String: Any] else {
                        throw NSError(domain: "unsupported_or_corrupt_image", code: 1)
                    }
                    let w = props[kCGImagePropertyPixelWidth as String] as? Int ?? 0
                    let h = props[kCGImagePropertyPixelHeight as String] as? Int ?? 0
                    guard w > 0, h > 0, w <= 100_000, h <= 100_000, Int64(w) * Int64(h) <= 200_000_000 else {
                        throw NSError(domain: "image_dimensions_limit", code: 1)
                    }
                    let orientation = props[kCGImagePropertyOrientation as String] as? Int ?? 1
                    result["width"] = orientation >= 5 ? h : w; result["height"] = orientation >= 5 ? w : h
                    let exif = props[kCGImagePropertyExifDictionary as String] as? [String: Any] ?? [:]
                    let tiff = props[kCGImagePropertyTIFFDictionary as String] as? [String: Any] ?? [:]
                    result["captured"] = exif[kCGImagePropertyExifDateTimeOriginal as String] as? String
                    result["offset"] = exif["OffsetTimeOriginal"] as? String
                    result["camera"] = (tiff[kCGImagePropertyTIFFModel as String] as? String).map { String($0.prefix(256)) }
                    if let gps = props[kCGImagePropertyGPSDictionary as String] as? [String: Any],
                       let latitude = gps[kCGImagePropertyGPSLatitude as String] as? Double,
                       let longitude = gps[kCGImagePropertyGPSLongitude as String] as? Double {
                        let lat = latitude * ((gps[kCGImagePropertyGPSLatitudeRef as String] as? String) == "S" ? -1 : 1)
                        let lon = longitude * ((gps[kCGImagePropertyGPSLongitudeRef as String] as? String) == "W" ? -1 : 1)
                        if abs(lat) <= 90 && abs(lon) <= 180 { result["latitude"] = lat; result["longitude"] = lon }
                    }
                    result["metadata_state"] = "ready"
                    if let output = req.output {
                        let options: [CFString: Any] = [kCGImageSourceCreateThumbnailFromImageAlways: true,
                            kCGImageSourceCreateThumbnailWithTransform: true, kCGImageSourceThumbnailMaxPixelSize: maxSize,
                            kCGImageSourceShouldCacheImmediately: true]
                        if let image = CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary) {
                            try writeJPEG(image, to: output); result["thumbnail_state"] = "ready"
                        } else { result["thumbnail_state"] = "failed"; result["error"] = "image_thumbnail_failed" }
                    }
                }
            }
            let output = try JSONSerialization.data(withJSONObject: result.compactMapValues { $0 }, options: [.sortedKeys])
            FileHandle.standardOutput.write(output)
        } catch {
            let code = (error as NSError).domain
            let safe = ["not_downloaded", "image_dimensions_limit", "local_folder_required", "unsupported_or_corrupt_image"].contains(code) ? code : "media_operation_failed"
            let output = try! JSONSerialization.data(withJSONObject: ["error": safe, "metadata_state": "failed", "thumbnail_state": "failed"])
            FileHandle.standardOutput.write(output)
        }
    }
}
