import Foundation
import CoreGraphics
import ImageIO
import UniformTypeIdentifiers
import AVFoundation
import CoreVideo

@main struct Fixtures {
    static func main() async throws {
        let root=URL(fileURLWithPath:CommandLine.arguments[1]);try FileManager.default.createDirectory(at:root,withIntermediateDirectories:true)
        let space=CGColorSpace(name:CGColorSpace.sRGB)!
        let ctx=CGContext(data:nil,width:640,height:480,bitsPerComponent:8,bytesPerRow:0,space:space,bitmapInfo:CGImageAlphaInfo.premultipliedLast.rawValue)!
        ctx.setFillColor(CGColor(red:0.70,green:0.79,blue:0.66,alpha:1));ctx.fill(CGRect(x:0,y:0,width:640,height:480))
        ctx.setFillColor(CGColor(red:0.92,green:0.80,blue:0.48,alpha:1));ctx.fillEllipse(in:CGRect(x:440,y:290,width:90,height:90))
        ctx.setFillColor(CGColor(red:0.30,green:0.48,blue:0.37,alpha:1));ctx.move(to:CGPoint(x:0,y:0));ctx.addLine(to:CGPoint(x:290,y:360));ctx.addLine(to:CGPoint(x:640,y:0));ctx.fillPath()
        let image=ctx.makeImage()!
        for (ext,type) in [("jpg",UTType.jpeg),("png",UTType.png),("heic",UTType.heic)] {
            let url=root.appendingPathComponent("mountain.\(ext)")
            let dest=CGImageDestinationCreateWithURL(url as CFURL,type.identifier as CFString,1,nil)!
            let calendar=Calendar.current;let today=Date();let year=calendar.component(.year,from:today)-2
            let captured=String(format:"%04d:%02d:%02d 14:30:00",year,calendar.component(.month,from:today),calendar.component(.day,from:today))
            let props:[CFString:Any]=[kCGImagePropertyOrientation:6,kCGImagePropertyExifDictionary:[kCGImagePropertyExifDateTimeOriginal:captured],kCGImagePropertyTIFFDictionary:[kCGImagePropertyTIFFModel:"Bimb Test Camera"],kCGImagePropertyGPSDictionary:[kCGImagePropertyGPSLatitude:12.97,kCGImagePropertyGPSLatitudeRef:"N",kCGImagePropertyGPSLongitude:77.59,kCGImagePropertyGPSLongitudeRef:"E"]]
            CGImageDestinationAddImage(dest,image,props as CFDictionary);guard CGImageDestinationFinalize(dest)else{fatalError("Image fixture failed")}
        }
        for (name,codec) in [("motion-h264.mp4",AVVideoCodecType.h264),("motion-hevc.mov",AVVideoCodecType.hevc)] {
            let url=root.appendingPathComponent(name);try? FileManager.default.removeItem(at:url)
            let writer=try AVAssetWriter(outputURL:url,fileType:name.hasSuffix("mp4") ? .mp4 : .mov)
            let input=AVAssetWriterInput(mediaType:.video,outputSettings:[AVVideoCodecKey:codec,AVVideoWidthKey:320,AVVideoHeightKey:240])
            let adapter=AVAssetWriterInputPixelBufferAdaptor(assetWriterInput:input,sourcePixelBufferAttributes:[kCVPixelBufferPixelFormatTypeKey as String:kCVPixelFormatType_32ARGB,kCVPixelBufferWidthKey as String:320,kCVPixelBufferHeightKey as String:240,kCVPixelBufferCGImageCompatibilityKey as String:true,kCVPixelBufferCGBitmapContextCompatibilityKey as String:true])
            writer.add(input);writer.startWriting();writer.startSession(atSourceTime:.zero)
            for frame in 0..<24 {
                while !input.isReadyForMoreMediaData {try await Task.sleep(nanoseconds:10_000_000)}
                var buffer:CVPixelBuffer?;CVPixelBufferPoolCreatePixelBuffer(nil,adapter.pixelBufferPool!,&buffer)
                let pixel=buffer!;CVPixelBufferLockBaseAddress(pixel,[])
                let canvas=CGContext(data:CVPixelBufferGetBaseAddress(pixel),width:320,height:240,bitsPerComponent:8,bytesPerRow:CVPixelBufferGetBytesPerRow(pixel),space:space,bitmapInfo:CGImageAlphaInfo.noneSkipFirst.rawValue)!
                canvas.draw(image,in:CGRect(x:0,y:0,width:320,height:240));canvas.setFillColor(CGColor(red:0.9,green:0.4,blue:0.2,alpha:1));canvas.fillEllipse(in:CGRect(x:frame*10,y:80,width:25,height:25));CVPixelBufferUnlockBaseAddress(pixel,[])
                guard adapter.append(pixel,withPresentationTime:CMTime(value:Int64(frame),timescale:12))else{throw writer.error!}
            }
            input.markAsFinished();await writer.finishWriting();guard writer.status == .completed else{throw writer.error!}
        }
        try Data("deliberately invalid image".utf8).write(to:root.appendingPathComponent("broken.jpg"))
        print("Generated JPEG, PNG, HEIC, H.264, HEVC and corrupt-media fixtures")
    }
}
