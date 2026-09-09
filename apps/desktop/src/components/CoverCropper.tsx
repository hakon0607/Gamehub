import { useCallback, useEffect, useRef, useState } from 'react';

/** Library covers are 2:3, so the crop box is locked to that shape. */
const ASPECT = 2 / 3;
const OUTPUT_WIDTH = 600;
const OUTPUT_HEIGHT = 900;
const MIN_BOX = 0.08;

interface Box {
  /** All four are fractions of the image's own width and height. */
  x: number;
  y: number;
  w: number;
  h: number;
}

const HANDLES: { id: string; x: number; y: number; cursor: string }[] = [
  { id: 'nw', x: 0, y: 0, cursor: 'nwse-resize' },
  { id: 'ne', x: 1, y: 0, cursor: 'nesw-resize' },
  { id: 'sw', x: 0, y: 1, cursor: 'nesw-resize' },
  { id: 'se', x: 1, y: 1, cursor: 'nwse-resize' },
];

/**
 * Crops an image down to a cover.
 *
 * The whole thing happens in the browser: the image arrives as a data URL from
 * Rust, the crop is drawn onto a canvas, and only the finished 600×900 PNG goes
 * back. Nothing here touches the filesystem.
 */
export function CoverCropper({
  dataUrl,
  gameName,
  onCancel,
  onSave,
}: {
  dataUrl: string;
  gameName: string;
  onCancel: () => void;
  onSave: (pngDataUrl: string) => void;
}) {
  const [image, setImage] = useState<HTMLImageElement | null>(null);
  const [box, setBox] = useState<Box>({ x: 0.2, y: 0.05, w: 0.6, h: 0.9 });
  const [saving, setSaving] = useState(false);
  const frameRef = useRef<HTMLDivElement>(null);

  // The starting box is the largest 2:3 rectangle that fits, centred.
  useEffect(() => {
    const img = new Image();
    img.onload = () => {
      const imageAspect = img.width / img.height;
      const w = imageAspect > ASPECT ? (ASPECT / imageAspect) : 1;
      const h = imageAspect > ASPECT ? 1 : (imageAspect / ASPECT);
      setImage(img);
      setBox({ x: (1 - w) / 2, y: (1 - h) / 2, w, h });
    };
    img.src = dataUrl;
  }, [dataUrl]);

  /** Keeps the box 2:3 in *pixel* terms, which needs the image's own aspect. */
  const heightFor = useCallback(
    (widthFraction: number) => {
      if (!image) return widthFraction;
      return (widthFraction * image.width) / ASPECT / image.height;
    },
    [image],
  );

  const startDrag = (handle: string) => (event: React.PointerEvent) => {
    event.preventDefault();
    event.stopPropagation();
    const frame = frameRef.current;
    if (!frame) return;
    const rect = frame.getBoundingClientRect();
    const origin = { ...box };
    const startX = event.clientX;
    const startY = event.clientY;

    const onMove = (e: PointerEvent) => {
      const dx = (e.clientX - startX) / rect.width;
      const dy = (e.clientY - startY) / rect.height;

      if (handle === 'move') {
        setBox({
          ...origin,
          x: Math.min(Math.max(0, origin.x + dx), 1 - origin.w),
          y: Math.min(Math.max(0, origin.y + dy), 1 - origin.h),
        });
        return;
      }

      // Corners resize from the opposite corner, so the shape stays anchored.
      const growing = handle.includes('e') ? dx : -dx;
      let w = Math.max(MIN_BOX, Math.min(1, origin.w + growing));
      let h = heightFor(w);
      if (h > 1) {
        h = 1;
        w = (h * image!.height * ASPECT) / image!.width;
      }
      const x = handle.includes('w') ? origin.x + origin.w - w : origin.x;
      const y = handle.includes('n') ? origin.y + origin.h - h : origin.y;
      setBox({
        x: Math.min(Math.max(0, x), 1 - w),
        y: Math.min(Math.max(0, y), 1 - h),
        w,
        h,
      });
    };

    const onUp = () => {
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onUp);
    };
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
  };

  const save = () => {
    if (!image) return;
    setSaving(true);
    const canvas = document.createElement('canvas');
    canvas.width = OUTPUT_WIDTH;
    canvas.height = OUTPUT_HEIGHT;
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      setSaving(false);
      return;
    }
    ctx.imageSmoothingQuality = 'high';
    ctx.drawImage(
      image,
      box.x * image.width,
      box.y * image.height,
      box.w * image.width,
      box.h * image.height,
      0,
      0,
      OUTPUT_WIDTH,
      OUTPUT_HEIGHT,
    );
    onSave(canvas.toDataURL('image/png'));
  };

  const pct = (value: number) => `${(value * 100).toFixed(3)}%`;

  return (
    <div className="scrim" role="dialog" aria-modal="true" aria-label={`Choose a cover for ${gameName}`}>
      <div className="dialog" style={{ width: 'min(620px, calc(100vw - 48px))' }}>
        <h2>Crop the cover</h2>
        <p className="version-note">
          Drag the box to choose what to keep. Covers are 2:3, so the box stays that shape.
        </p>

        <div className="cropper" ref={frameRef}>
          {image && <img src={dataUrl} alt="" draggable={false} />}
          {image && (
            <>
              <div className="crop-shade" style={{ inset: `0 0 ${pct(1 - box.y)} 0` }} />
              <div className="crop-shade" style={{ inset: `${pct(box.y + box.h)} 0 0 0` }} />
              <div
                className="crop-shade"
                style={{ inset: `${pct(box.y)} ${pct(1 - box.x)} ${pct(1 - box.y - box.h)} 0` }}
              />
              <div
                className="crop-shade"
                style={{ inset: `${pct(box.y)} 0 ${pct(1 - box.y - box.h)} ${pct(box.x + box.w)}` }}
              />
              <div
                className="crop-box"
                onPointerDown={startDrag('move')}
                style={{ left: pct(box.x), top: pct(box.y), width: pct(box.w), height: pct(box.h) }}
              />
              {HANDLES.map((handle) => (
                <span
                  key={handle.id}
                  className="crop-handle"
                  onPointerDown={startDrag(handle.id)}
                  style={{
                    left: pct(box.x + box.w * handle.x),
                    top: pct(box.y + box.h * handle.y),
                    cursor: handle.cursor,
                  }}
                />
              ))}
            </>
          )}
        </div>

        <div className="dialog-actions" style={{ marginTop: 18 }}>
          <button className="btn" onClick={onCancel} disabled={saving}>
            Cancel
          </button>
          <button className="btn btn-accent" onClick={save} disabled={!image || saving}>
            {saving ? 'Saving…' : 'Use this cover'}
          </button>
        </div>
      </div>
    </div>
  );
}
