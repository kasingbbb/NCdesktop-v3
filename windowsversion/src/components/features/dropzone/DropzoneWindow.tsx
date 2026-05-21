import { useCallback, useEffect, useState } from "react";
import { useDropzoneStore } from "../../../stores/dropzoneStore";
import { DropzoneIdle } from "./DropzoneIdle";
import { DropzoneProcessing } from "./DropzoneProcessing";
import { DropzoneComplete } from "./DropzoneComplete";
import { DropzoneExpanded } from "./DropzoneExpanded";

export function DropzoneWindow() {
  const phase = useDropzoneStore((s) => s.phase);
  const isExpanded = useDropzoneStore((s) => s.isExpanded);
  const setPhase = useDropzoneStore((s) => s.setPhase);
  const [isDragOver, setIsDragOver] = useState(false);

  const handleFileDrop = useCallback((): void => {
    setTimeout(() => setPhase("complete"), 1500);
    setTimeout(() => setPhase("idle"), 3500);
  }, [setPhase]);

  const handleDragEnter = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      setIsDragOver(true);
      setPhase("attract");
    },
    [setPhase]
  );

  const handleDragLeave = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      if (e.currentTarget.contains(e.relatedTarget as Node)) return;
      setIsDragOver(false);
      setPhase("idle");
    },
    [setPhase]
  );

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
  }, []);

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      setIsDragOver(false);
      setPhase("processing");

      const files = Array.from(e.dataTransfer.files);
      if (files.length > 0) {
        handleFileDrop();
      }
    },
    [setPhase, handleFileDrop]
  );

  useEffect(() => {
    setPhase("idle");
  }, [setPhase]);

  const scale = isDragOver ? 1.3 : 1;

  return (
    <div
      className="w-full h-full flex items-center justify-center"
      style={{
        background: "transparent",
        transition: "transform var(--duration-normal) var(--ease-out-expo)",
        transform: `scale(${scale})`,
      }}
      onDragEnter={handleDragEnter}
      onDragLeave={handleDragLeave}
      onDragOver={handleDragOver}
      onDrop={handleDrop}
    >
      {isExpanded ? (
        <DropzoneExpanded />
      ) : (
        <>
          {phase === "idle" && <DropzoneIdle />}
          {phase === "attract" && <DropzoneIdle isAttract />}
          {phase === "processing" && <DropzoneProcessing />}
          {phase === "complete" && <DropzoneComplete />}
        </>
      )}
    </div>
  );
}
