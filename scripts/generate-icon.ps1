param(
    [string]$OutputDirectory = (Join-Path $PSScriptRoot '..\src-tauri\icons')
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

function New-ResizedIconBitmap {
    param(
        [System.Drawing.Image]$Source,
        [System.Drawing.Rectangle]$SourceCrop,
        [int]$Size
    )

    $bitmap = [System.Drawing.Bitmap]::new($Size, $Size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $bitmap.SetResolution(96, 96)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.Clear([System.Drawing.Color]::Transparent)
    $graphics.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceCopy
    $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $margin = if ($Size -le 20) {
        0
    } elseif ($Size -le 32) {
        1
    } elseif ($Size -le 48) {
        2
    } else {
        [Math]::Round($Size * 0.045)
    }
    $destination = [System.Drawing.Rectangle]::new(
        $margin,
        $margin,
        $Size - 2 * $margin,
        $Size - 2 * $margin
    )
    $graphics.DrawImage(
        $Source,
        $destination,
        $SourceCrop.X,
        $SourceCrop.Y,
        $SourceCrop.Width,
        $SourceCrop.Height,
        [System.Drawing.GraphicsUnit]::Pixel
    )
    $graphics.Dispose()
    return $bitmap
}

function ConvertTo-PngBytes {
    param([System.Drawing.Bitmap]$Bitmap)

    $stream = [System.IO.MemoryStream]::new()
    $Bitmap.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
    $bytes = $stream.ToArray()
    $stream.Dispose()
    return $bytes
}

$resolvedOutput = [System.IO.Path]::GetFullPath($OutputDirectory)
$masterPath = Join-Path $resolvedOutput 'icon-master.png'
if (-not (Test-Path -LiteralPath $masterPath)) {
    throw "Missing icon master: $masterPath"
}

[System.IO.Directory]::CreateDirectory($resolvedOutput) | Out-Null
$source = [System.Drawing.Image]::FromFile($masterPath)
if ($source.Width -ne $source.Height -or $source.Width -lt 1024) {
    $source.Dispose()
    throw 'The icon master must be a square image of at least 1024 x 1024 pixels.'
}

# Square optical crop around the approved artwork. The source pixels are not modified;
# only excess transparent canvas is removed before each Windows size is rendered.
$sourceCrop = [System.Drawing.Rectangle]::new(144, 131, 966, 966)

$sizes = @(16, 20, 24, 32, 40, 48, 64, 128, 256)
$entries = foreach ($size in $sizes) {
    $bitmap = New-ResizedIconBitmap -Source $source -SourceCrop $sourceCrop -Size $size
    $pngPath = Join-Path $resolvedOutput "icon-$size.png"
    $bitmap.Save($pngPath, [System.Drawing.Imaging.ImageFormat]::Png)
    $bytes = ConvertTo-PngBytes $bitmap
    $bitmap.Dispose()
    [PSCustomObject]@{ Size = $size; Bytes = $bytes }
}

$preview = New-ResizedIconBitmap -Source $source -SourceCrop $sourceCrop -Size 512
$preview.Save((Join-Path $resolvedOutput 'icon-preview.png'), [System.Drawing.Imaging.ImageFormat]::Png)
$preview.Dispose()
$source.Dispose()

$iconPath = Join-Path $resolvedOutput 'icon.ico'
$stream = [System.IO.File]::Create($iconPath)
$writer = [System.IO.BinaryWriter]::new($stream)
$writer.Write([uint16]0)
$writer.Write([uint16]1)
$writer.Write([uint16]$entries.Count)
$offset = 6 + 16 * $entries.Count

foreach ($entry in $entries) {
    $writer.Write([byte]$(if ($entry.Size -eq 256) { 0 } else { $entry.Size }))
    $writer.Write([byte]$(if ($entry.Size -eq 256) { 0 } else { $entry.Size }))
    $writer.Write([byte]0)
    $writer.Write([byte]0)
    $writer.Write([uint16]1)
    $writer.Write([uint16]32)
    $writer.Write([uint32]$entry.Bytes.Length)
    $writer.Write([uint32]$offset)
    $offset += $entry.Bytes.Length
}

foreach ($entry in $entries) {
    $writer.Write([byte[]]$entry.Bytes)
}

$writer.Dispose()
$stream.Dispose()

$publicDirectory = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\public'))
[System.IO.Directory]::CreateDirectory($publicDirectory) | Out-Null
Copy-Item -LiteralPath (Join-Path $resolvedOutput 'icon-64.png') -Destination (Join-Path $publicDirectory 'favicon.png') -Force
Write-Output "Generated $iconPath and PNG sizes: $($sizes -join ', ')"
