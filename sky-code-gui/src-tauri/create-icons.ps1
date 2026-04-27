Add-Type -AssemblyName System.Drawing

# Function to create icon
function New-PlaceholderIcon {
    param([int]$Size, [string]$OutPath)
    
    $bitmap = New-Object System.Drawing.Bitmap($Size, $Size)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    
    # Fill with blue background
    $brush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::DodgerBlue)
    $graphics.FillRectangle($brush, 0, 0, $Size, $Size)
    
    # Draw "S" letter (for Sky-Code)
    $font = New-Object System.Drawing.Font("Arial", [int]($Size * 0.6), [System.Drawing.FontStyle]::Bold)
    $textBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
    $format = New-Object System.Drawing.StringFormat
    $format.Alignment = [System.Drawing.StringAlignment]::Center
    $format.LineAlignment = [System.Drawing.StringAlignment]::Center
    
    $rect = New-Object System.Drawing.RectangleF(0, 0, $Size, $Size)
    $graphics.DrawString("S", $font, $textBrush, $rect, $format)
    
    $bitmap.Save($OutPath, [System.Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose()
    $bitmap.Dispose()
}

# Create PNG icons
New-PlaceholderIcon -Size 32 -OutPath "icons\32x32.png"
New-PlaceholderIcon -Size 128 -OutPath "icons\128x128.png"
New-PlaceholderIcon -Size 256 -OutPath "icons\128x128@2x.png"

Write-Host "PNG icons created successfully!"

# For ICO, we'll convert the 256x256 PNG
$png = [System.Drawing.Image]::FromFile("$PWD\icons\128x128@2x.png")
$icon = [System.Drawing.Icon]::FromHandle($png.GetHicon())

# Save as ICO
$icoStream = [System.IO.File]::Create("$PWD\icons\icon.ico")
$icon.Save($icoStream)
$icoStream.Close()

Write-Host "ICO icon created successfully!"

# For macOS ICNS, we'll just copy the PNG as placeholder (won't work on Mac but ok for Windows dev)
Copy-Item "icons\128x128@2x.png" "icons\icon.icns" -Force

Write-Host "All icons created!"
