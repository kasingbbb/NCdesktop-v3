import Foundation
import Vision
import CoreGraphics
import ImageIO
import PDFKit

@_cdecl("recognize_text_in_image")
public func recognizeTextInImage(_ pathPtr: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>? {
    let path = String(cString: pathPtr)

    let imageURL = URL(fileURLWithPath: path)
    guard let imageSource = CGImageSourceCreateWithURL(imageURL as CFURL, nil),
          let cgImage = CGImageSourceCreateImageAtIndex(imageSource, 0, nil) else {
        return makeResult(success: false, error: "无法加载图片: \(path)")
    }

    let request = VNRecognizeTextRequest()
    request.recognitionLevel = .accurate
    request.recognitionLanguages = ["zh-Hans", "zh-Hant", "en"]
    request.usesLanguageCorrection = true

    let handler = VNImageRequestHandler(cgImage: cgImage, options: [:])

    do {
        try handler.perform([request])
    } catch {
        return makeResult(success: false, error: "OCR 处理失败: \(error.localizedDescription)")
    }

    guard let observations = request.results else {
        return makeResult(success: true, results: [])
    }

    var results: [[String: Any]] = []
    for observation in observations {
        guard let candidate = observation.topCandidates(1).first else { continue }
        let bbox = observation.boundingBox
        results.append([
            "text": candidate.string,
            "confidence": candidate.confidence,
            "bbox": [bbox.origin.x, bbox.origin.y, bbox.width, bbox.height]
        ])
    }

    return makeResult(success: true, results: results)
}

@_cdecl("free_rust_string")
public func freeRustString(_ ptr: UnsafeMutablePointer<CChar>?) {
    if let ptr = ptr {
        free(ptr)
    }
}

@_cdecl("recognize_text_in_pdf_page")
public func recognizeTextInPdfPage(_ pathPtr: UnsafePointer<CChar>, _ pageIndex: Int32) -> UnsafeMutablePointer<CChar>? {
    let path = String(cString: pathPtr)
    let url = URL(fileURLWithPath: path)

    guard let pdfDoc = PDFDocument(url: url) else {
        return makeResult(success: false, error: "无法打开 PDF: \(path)")
    }

    guard let page = pdfDoc.page(at: Int(pageIndex)) else {
        return makeResult(success: false, error: "无效页码: \(pageIndex)")
    }

    let pageRect = page.bounds(for: .mediaBox)
    let scale: CGFloat = 2.0
    let width = Int(pageRect.width * scale)
    let height = Int(pageRect.height * scale)

    guard let colorSpace = CGColorSpace(name: CGColorSpace.sRGB),
          let context = CGContext(data: nil, width: width, height: height,
                                  bitsPerComponent: 8, bytesPerRow: 0,
                                  space: colorSpace,
                                  bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else {
        return makeResult(success: false, error: "无法创建绘图上下文")
    }

    context.setFillColor(CGColor.white)
    context.fill(CGRect(x: 0, y: 0, width: width, height: height))
    context.scaleBy(x: scale, y: scale)

    page.draw(with: .mediaBox, to: context)

    guard let cgImage = context.makeImage() else {
        return makeResult(success: false, error: "无法渲染页面为图片")
    }

    let request = VNRecognizeTextRequest()
    request.recognitionLevel = .accurate
    request.recognitionLanguages = ["zh-Hans", "zh-Hant", "en"]
    request.usesLanguageCorrection = true

    let handler = VNImageRequestHandler(cgImage: cgImage, options: [:])

    do {
        try handler.perform([request])
    } catch {
        return makeResult(success: false, error: "OCR 失败: \(error.localizedDescription)")
    }

    guard let observations = request.results else {
        return makeResult(success: true, results: [])
    }

    var results: [[String: Any]] = []
    for observation in observations {
        guard let candidate = observation.topCandidates(1).first else { continue }
        let bbox = observation.boundingBox
        results.append([
            "text": candidate.string,
            "confidence": candidate.confidence,
            "bbox": [bbox.origin.x, bbox.origin.y, bbox.width, bbox.height]
        ])
    }

    return makeResult(success: true, results: results)
}

@_cdecl("get_pdf_page_count")
public func getPdfPageCount(_ pathPtr: UnsafePointer<CChar>) -> Int32 {
    let path = String(cString: pathPtr)
    let url = URL(fileURLWithPath: path)
    guard let pdfDoc = PDFDocument(url: url) else { return -1 }
    return Int32(pdfDoc.pageCount)
}

private func makeResult(success: Bool, results: [[String: Any]] = [], error: String? = nil) -> UnsafeMutablePointer<CChar>? {
    var dict: [String: Any] = ["success": success]
    if !results.isEmpty { dict["results"] = results }
    if let error = error { dict["error"] = error }

    guard let data = try? JSONSerialization.data(withJSONObject: dict),
          let json = String(data: data, encoding: .utf8) else {
        return strdup("{\"success\":false,\"error\":\"JSON 序列化失败\"}")
    }
    return strdup(json)
}
