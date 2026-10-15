SheetPacker 0.1.0  -  flipbook / sprite sheet packer
====================================================

Portable: just run SheetPacker.exe. No installer, no dependencies.

If Windows SmartScreen warns ("Windows protected your PC"), it's only because
the file was downloaded with the "from the internet" mark. Either click
"More info -> Run anyway", or unblock it first (right-click SheetPacker.exe
-> Properties -> Unblock), or download it with PowerShell:
    Invoke-WebRequest <url> -OutFile SheetPacker.zip
(Invoke-WebRequest doesn't set that mark, so no warning.)

GUI
  1. Add files...  or  Import folder...  (or drag images onto the window)
  2. Frames are listed on the left. DRAG a frame to reorder. Click x to remove.
  3. Set Columns / Rows and the Sheet size (Width/Height/Margin/Spacing).
  4. Fit: Exact (default, no scaling) / Letterbox (keep aspect) / Stretch.
  5. Play button animates the frames to preview the flipbook.
  6. Export sheet... -> saves a PNG.

Order is by file name (frame_2.png before frame_10.png). If there are more
frames than cells, an "Auto-fit grid" button appears.

CLI (optional)
  SheetPacker.exe <folder or files> --out sheet.png --cols 4 --rows 4 --size 2048x2048 --fit exact
