import { memo } from "react";
import { Loader2, AlertCircle, Ban, CheckCircle2 } from "lucide-react";
import type { ExtractionStatus } from "../../../types/extraction";

interface ExtractionBadgeProps {
  status: ExtractionStatus | undefined;
  size?: "sm" | "md";
}

export const ExtractionBadge = memo(function ExtractionBadge({
  status,
  size = "sm",
}: ExtractionBadgeProps) {
  if (!status || status === "pending") return null;

  const sizeClass = size === "sm" ? "w-3.5 h-3.5" : "w-4 h-4";

  switch (status) {
    case "extracting":
      return <Loader2 className={`${sizeClass} text-blue-500 animate-spin`} />;
    case "extracted":
      return <CheckCircle2 className={`${sizeClass} text-green-500`} />;
    case "failed":
      return <AlertCircle className={`${sizeClass} text-red-500`} />;
    case "unsupported":
      return <Ban className={`${sizeClass} text-gray-400`} />;
    default:
      return null;
  }
});
