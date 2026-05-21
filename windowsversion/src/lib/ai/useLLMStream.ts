import { useCallback, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";

interface UseLLMStreamReturn {
  content: string;
  isStreaming: boolean;
  error: string | null;
  startStream: (eventName: string) => Promise<void>;
  cancel: () => void;
  reset: () => void;
}

export function useLLMStream(): UseLLMStreamReturn {
  const [content, setContent] = useState("");
  const [isStreaming, setIsStreaming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const unlistenRef = useRef<(() => void) | null>(null);

  const startStream = useCallback(async (eventName: string) => {
    setContent("");
    setIsStreaming(true);
    setError(null);

    try {
      const unlisten = await listen<string>(eventName, (event) => {
        if (event.payload === "[DONE]") {
          setIsStreaming(false);
          unlisten();
          return;
        }
        if (event.payload.startsWith("[ERROR]")) {
          setError(event.payload.replace("[ERROR] ", ""));
          setIsStreaming(false);
          unlisten();
          return;
        }
        setContent((prev) => prev + event.payload);
      });

      unlistenRef.current = unlisten;
    } catch (e) {
      setError(String(e));
      setIsStreaming(false);
    }
  }, []);

  const cancel = useCallback(() => {
    if (unlistenRef.current) {
      unlistenRef.current();
      unlistenRef.current = null;
    }
    setIsStreaming(false);
  }, []);

  const reset = useCallback(() => {
    cancel();
    setContent("");
    setError(null);
  }, [cancel]);

  return { content, isStreaming, error, startStream, cancel, reset };
}
