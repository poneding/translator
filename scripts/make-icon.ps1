Add-Type -AssemblyName System.Drawing

$size = 1024
$bmp = New-Object System.Drawing.Bitmap $size, $size
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit

# Soft gradient background (top-left teal -> bottom-right slate)
$rect = New-Object System.Drawing.Rectangle 0, 0, $size, $size
$brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush $rect, ([System.Drawing.Color]::FromArgb(255, 16, 132, 145)), ([System.Drawing.Color]::FromArgb(255, 14, 30, 49)), 135.0

$g.FillRectangle($brush, $rect)
$brush.Dispose()

# Subtle dotted overlay for texture
$overlayPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(20, 255, 255, 255)), 1
for ($y = 0; $y -lt $size; $y += 32) {
    for ($x = 0; $x -lt $size; $x += 32) {
        $g.FillEllipse([System.Drawing.Brushes]::WhiteSmoke, $x - 0.5, $y - 0.5, 1, 1)
    }
}
$overlayPen.Dispose()

# Rounded "T" letter (translator)
$char = "T"
$fontFamily = New-Object System.Drawing.FontFamily "Segoe UI"
$font = New-Object System.Drawing.Font $fontFamily, 720, ([System.Drawing.FontStyle]::Bold), ([System.Drawing.GraphicsUnit]::Pixel)
$textBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 240, 244, 248))
$shadow = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(60, 0, 0, 0))
$sf = New-Object System.Drawing.StringFormat
$sf.Alignment = [System.Drawing.StringAlignment]::Center
$sf.LineAlignment = [System.Drawing.StringAlignment]::Center

$shadowRect = New-Object System.Drawing.RectangleF 0, 30, $size, $size
$g.DrawString($char, $font, $shadow, $shadowRect, $sf)

$textRect = New-Object System.Drawing.RectangleF 0, 0, $size, $size
$g.DrawString($char, $font, $textBrush, $textRect, $sf)
$textBrush.Dispose()
$shadow.Dispose()
$font.Dispose()
$fontFamily.Dispose()

# Outer subtle border
$border = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(120, 255, 255, 255)), 6
$g.DrawRectangle($border, 3, 3, $size - 6, $size - 6)
$border.Dispose()

$g.Dispose()
$out = "C:\Users\dp\Easydict\translator\crates\app\icons\app-icon-source.png"
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()

Get-Item $out | ForEach-Object { "Wrote: $($_.Name) ($([math]::Round($_.Length/1KB, 1)) KB, $($_.Width)x$($_.Height))" }
