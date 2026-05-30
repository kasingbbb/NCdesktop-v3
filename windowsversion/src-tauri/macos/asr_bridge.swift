import Foundation
import Speech

/// 将音频文件转写为文字，同步阻塞直至完成（使用 DispatchSemaphore）
/// - 超时：600 秒（适合 10 分钟以内录音）
/// - 返回 JSON：{"success": true, "text": "..."} 或 {"success": false, "error": "..."}
@_cdecl("transcribe_audio_file")
public func transcribeAudioFile(_ pathPtr: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>? {
    let path = String(cString: pathPtr)
    let url = URL(fileURLWithPath: path)

    // 检查文件是否存在
    guard FileManager.default.fileExists(atPath: path) else {
        return makeAsrResult(success: false, error: "音频文件不存在: \(path)")
    }

    // macOS 26+ 的 TCC 只为真正的 .app bundle 读取 Info.plist；
    // 裸 Mach-O（tauri dev 模式）即便通过 __TEXT,__info_plist 段嵌入了
    // NSSpeechRecognitionUsageDescription，调用 SFSpeechRecognizer 仍会被
    // TCC 以 SIGABRT 直接杀进程。此处在 dev 模式下优雅返回，而非让进程闪退。
    guard Bundle.main.bundleURL.pathExtension == "app" else {
        return makeAsrResult(
            success: false,
            error: "语音识别仅在正式构建（.app）中可用；开发模式下已跳过以避免被系统隐私保护终止"
        )
    }

    // 请求授权（同步等待）
    let authStatus = SFSpeechRecognizer.authorizationStatus()
    if authStatus == .notDetermined {
        let authSemaphore = DispatchSemaphore(value: 0)
        SFSpeechRecognizer.requestAuthorization { _ in authSemaphore.signal() }
        authSemaphore.wait()
    }

    guard SFSpeechRecognizer.authorizationStatus() == .authorized else {
        return makeAsrResult(success: false, error: "语音识别未获授权，请在系统偏好设置中允许")
    }

    // 尝试中文识别器，失败则回退到英文
    guard let recognizer = SFSpeechRecognizer(locale: Locale(identifier: "zh-CN"))
                        ?? SFSpeechRecognizer(locale: Locale(identifier: "en-US")) else {
        return makeAsrResult(success: false, error: "无法初始化语音识别器")
    }

    guard recognizer.isAvailable else {
        return makeAsrResult(success: false, error: "语音识别器当前不可用（可能需要网络或设备不支持）")
    }

    let request = SFSpeechURLRecognitionRequest(url: url)
    request.shouldReportPartialResults = false

    let semaphore = DispatchSemaphore(value: 0)
    var finalTranscription = ""
    var errorMessage: String? = nil
    var signalled = false

    recognizer.recognitionTask(with: request) { result, error in
        guard !signalled else { return }

        if let result = result {
            // 保存最新转写结果（partial 也缓存，以防 isFinal 未触发）
            finalTranscription = result.bestTranscription.formattedString
            if result.isFinal {
                signalled = true
                semaphore.signal()
                return
            }
        }

        if let error = error {
            // error 不一定致命，但如果 finalTranscription 也为空则记录
            if finalTranscription.isEmpty {
                errorMessage = error.localizedDescription
            }
            if !signalled {
                signalled = true
                semaphore.signal()
            }
        }
    }

    // 最长等待 600 秒
    let deadline = DispatchTime.now() + .seconds(600)
    if semaphore.wait(timeout: deadline) == .timedOut {
        return makeAsrResult(success: false, error: "语音转写超时（超过 600 秒）")
    }

    if finalTranscription.isEmpty {
        let msg = errorMessage ?? "转写结果为空（可能音频无语音内容）"
        return makeAsrResult(success: false, error: msg)
    }

    return makeAsrResult(success: true, text: finalTranscription)
}

@_cdecl("free_asr_string")
public func freeAsrString(_ ptr: UnsafeMutablePointer<CChar>?) {
    if let ptr = ptr {
        free(ptr)
    }
}

private func makeAsrResult(success: Bool, text: String = "", error: String? = nil) -> UnsafeMutablePointer<CChar>? {
    var dict: [String: Any] = ["success": success]
    if !text.isEmpty { dict["text"] = text }
    if let error = error { dict["error"] = error }

    guard let data = try? JSONSerialization.data(withJSONObject: dict),
          let json = String(data: data, encoding: .utf8) else {
        return strdup("{\"success\":false,\"error\":\"JSON 序列化失败\"}")
    }
    return strdup(json)
}
