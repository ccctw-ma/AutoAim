#Requires -Version 5.1

[CmdletBinding()]
param(
    [string]$InitialFile
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version 2.0

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$InstallRoot = Split-Path -Parent $ScriptDir
$AutoAimExe = Join-Path $InstallRoot "bin\autoaim.exe"
$SampleFile = Join-Path $InstallRoot "examples\sample_frames.jsonl"
$OutputDir = Join-Path $InstallRoot ".e2e-output"
$DefaultEventsPath = Join-Path $OutputDir "events.jsonl"

$Colors = @{
    Background = [System.Drawing.Color]::FromArgb(12, 18, 30)
    Panel = [System.Drawing.Color]::FromArgb(18, 26, 42)
    PanelAlt = [System.Drawing.Color]::FromArgb(24, 34, 54)
    Text = [System.Drawing.Color]::FromArgb(226, 232, 240)
    Muted = [System.Drawing.Color]::FromArgb(148, 163, 184)
    Accent = [System.Drawing.Color]::FromArgb(34, 211, 238)
    Warn = [System.Drawing.Color]::FromArgb(249, 115, 22)
    Border = [System.Drawing.Color]::FromArgb(51, 65, 85)
}

function New-Font {
    param(
        [float]$Size,
        [System.Drawing.FontStyle]$Style = [System.Drawing.FontStyle]::Regular
    )

    New-Object System.Drawing.Font -ArgumentList "Segoe UI", $Size, $Style
}

function New-Label {
    param(
        [string]$Text,
        [int]$X,
        [int]$Y,
        [int]$Width,
        [int]$Height,
        [float]$Size = 9,
        [System.Drawing.Color]$Color = $Colors.Text,
        [System.Drawing.FontStyle]$Style = [System.Drawing.FontStyle]::Regular
    )

    $label = New-Object System.Windows.Forms.Label
    $label.Text = $Text
    $label.Location = New-Object System.Drawing.Point -ArgumentList $X, $Y
    $label.Size = New-Object System.Drawing.Size -ArgumentList $Width, $Height
    $label.ForeColor = $Color
    $label.BackColor = [System.Drawing.Color]::Transparent
    $label.Font = New-Font -Size $Size -Style $Style
    $label
}

function New-Button {
    param(
        [string]$Text,
        [int]$X,
        [int]$Y,
        [int]$Width,
        [int]$Height
    )

    $button = New-Object System.Windows.Forms.Button
    $button.Text = $Text
    $button.Location = New-Object System.Drawing.Point -ArgumentList $X, $Y
    $button.Size = New-Object System.Drawing.Size -ArgumentList $Width, $Height
    $button.FlatStyle = [System.Windows.Forms.FlatStyle]::Flat
    $button.FlatAppearance.BorderColor = $Colors.Border
    $button.BackColor = $Colors.PanelAlt
    $button.ForeColor = $Colors.Text
    $button.Font = New-Font -Size 9 -Style ([System.Drawing.FontStyle]::Bold)
    $button
}

function New-TextBox {
    param(
        [int]$X,
        [int]$Y,
        [int]$Width,
        [int]$Height,
        [switch]$Multiline
    )

    $textBox = New-Object System.Windows.Forms.TextBox
    $textBox.Location = New-Object System.Drawing.Point -ArgumentList $X, $Y
    $textBox.Size = New-Object System.Drawing.Size -ArgumentList $Width, $Height
    $textBox.BackColor = [System.Drawing.Color]::FromArgb(8, 13, 24)
    $textBox.ForeColor = $Colors.Text
    $textBox.BorderStyle = [System.Windows.Forms.BorderStyle]::FixedSingle
    $textBox.Font = New-Font -Size 9
    if ($Multiline) {
        $textBox.Multiline = $true
        $textBox.ScrollBars = [System.Windows.Forms.ScrollBars]::Both
        $textBox.WordWrap = $false
    }
    $textBox
}

function Quote-Argument {
    param([string]$Value)
    '"' + ($Value -replace '"', '\"') + '"'
}

function Invoke-AutoAim {
    param([string[]]$Arguments)

    if (-not (Test-Path $AutoAimExe -PathType Leaf)) {
        throw "Cannot find CLI executable: $AutoAimExe"
    }

    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $AutoAimExe
    $psi.Arguments = ($Arguments | ForEach-Object { Quote-Argument $_ }) -join " "
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true

    $process = [System.Diagnostics.Process]::Start($psi)
    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    $process.WaitForExit()

    [PSCustomObject]@{
        ExitCode = $process.ExitCode
        Stdout = $stdout
        Stderr = $stderr
        Command = "$AutoAimExe $($psi.Arguments)"
    }
}

function Set-Status {
    param([string]$Message)
    $statusLabel.Text = $Message
}

function Append-Log {
    param([string]$Message)
    $outputBox.AppendText($Message.TrimEnd() + [Environment]::NewLine)
}

function Assert-InputFile {
    $path = $inputBox.Text.Trim()
    if ([string]::IsNullOrWhiteSpace($path) -or -not (Test-Path $path -PathType Leaf)) {
        throw "Select a valid JSONL frame file first."
    }
    $path
}

$form = New-Object System.Windows.Forms.Form
$form.Text = "AutoAim Review"
$form.StartPosition = [System.Windows.Forms.FormStartPosition]::CenterScreen
$form.Size = New-Object System.Drawing.Size -ArgumentList 1080, 720
$form.MinimumSize = New-Object System.Drawing.Size -ArgumentList 980, 640
$form.BackColor = $Colors.Background
$form.Font = New-Font -Size 9

$header = New-Object System.Windows.Forms.Panel
$header.Location = New-Object System.Drawing.Point -ArgumentList 0, 0
$header.Size = New-Object System.Drawing.Size -ArgumentList 1080, 118
$header.Anchor = "Top,Left,Right"
$header.BackColor = $Colors.Background
$form.Controls.Add($header)

$logoPanel = New-Object System.Windows.Forms.Panel
$logoPanel.Location = New-Object System.Drawing.Point -ArgumentList 28, 22
$logoPanel.Size = New-Object System.Drawing.Size -ArgumentList 74, 74
$logoPanel.BackColor = [System.Drawing.Color]::Transparent
$logoPanel.Add_Paint({
    param($sender, $event)
    $g = $event.Graphics
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $bounds = New-Object System.Drawing.Rectangle -ArgumentList 5, 5, 64, 64
    $bgBrush = New-Object System.Drawing.SolidBrush -ArgumentList ([System.Drawing.Color]::FromArgb(18, 26, 42))
    $accentPen = New-Object System.Drawing.Pen -ArgumentList $Colors.Accent, 4
    $mutedPen = New-Object System.Drawing.Pen -ArgumentList $Colors.Border, 3
    $warnBrush = New-Object System.Drawing.SolidBrush -ArgumentList $Colors.Warn
    $textBrush = New-Object System.Drawing.SolidBrush -ArgumentList $Colors.Text
    $font = New-Font -Size 20 -Style ([System.Drawing.FontStyle]::Bold)

    $g.FillEllipse($bgBrush, $bounds)
    $g.DrawEllipse($mutedPen, $bounds)
    $g.DrawEllipse($accentPen, 18, 18, 38, 38)
    $g.DrawLine($accentPen, 37, 5, 37, 18)
    $g.DrawLine($accentPen, 37, 56, 37, 69)
    $g.DrawLine($accentPen, 5, 37, 18, 37)
    $g.DrawLine($accentPen, 56, 37, 69, 37)
    $g.DrawString("A", $font, $textBrush, 25, 20)
    $g.FillEllipse($warnBrush, 34, 34, 8, 8)

    $bgBrush.Dispose()
    $accentPen.Dispose()
    $mutedPen.Dispose()
    $warnBrush.Dispose()
    $textBrush.Dispose()
    $font.Dispose()
})
$header.Controls.Add($logoPanel)

$header.Controls.Add((New-Label -Text "AutoAim Review" -X 118 -Y 24 -Width 420 -Height 34 -Size 20 -Style ([System.Drawing.FontStyle]::Bold)))
$header.Controls.Add((New-Label -Text "Visualization-only review workspace. No mouse movement, clicks, process attach, or game control." -X 120 -Y 62 -Width 760 -Height 22 -Size 9 -Color $Colors.Muted))
$header.Controls.Add((New-Label -Text "Offline JSONL runtime is available now. Live capture / ONNX / overlay are staged next." -X 120 -Y 84 -Width 760 -Height 22 -Size 9 -Color $Colors.Warn))

$mainPanel = New-Object System.Windows.Forms.Panel
$mainPanel.Location = New-Object System.Drawing.Point -ArgumentList 22, 126
$mainPanel.Size = New-Object System.Drawing.Size -ArgumentList 1020, 504
$mainPanel.Anchor = "Top,Bottom,Left,Right"
$mainPanel.BackColor = $Colors.Panel
$form.Controls.Add($mainPanel)

$mainPanel.Controls.Add((New-Label -Text "Frame JSONL" -X 24 -Y 24 -Width 160 -Height 22 -Size 10 -Style ([System.Drawing.FontStyle]::Bold)))
$inputBox = New-TextBox -X 24 -Y 52 -Width 650 -Height 28
$inputBox.Anchor = "Top,Left,Right"
if ($InitialFile) {
    $inputBox.Text = $InitialFile
}
elseif (Test-Path $SampleFile -PathType Leaf) {
    $inputBox.Text = $SampleFile
}
$mainPanel.Controls.Add($inputBox)

$selectButton = New-Button -Text "Select..." -X 690 -Y 50 -Width 110 -Height 32
$selectButton.Anchor = "Top,Right"
$mainPanel.Controls.Add($selectButton)

$sampleButton = New-Button -Text "Use Sample" -X 812 -Y 50 -Width 120 -Height 32
$sampleButton.Anchor = "Top,Right"
$mainPanel.Controls.Add($sampleButton)

$mainPanel.Controls.Add((New-Label -Text "Output events" -X 24 -Y 94 -Width 160 -Height 22 -Size 10 -Style ([System.Drawing.FontStyle]::Bold)))
$outputPathBox = New-TextBox -X 24 -Y 122 -Width 650 -Height 28
$outputPathBox.Anchor = "Top,Left,Right"
$outputPathBox.Text = $DefaultEventsPath
$mainPanel.Controls.Add($outputPathBox)

$outputBrowseButton = New-Button -Text "Save As..." -X 690 -Y 120 -Width 110 -Height 32
$outputBrowseButton.Anchor = "Top,Right"
$mainPanel.Controls.Add($outputBrowseButton)

$openFolderButton = New-Button -Text "Open Folder" -X 812 -Y 120 -Width 120 -Height 32
$openFolderButton.Anchor = "Top,Right"
$mainPanel.Controls.Add($openFolderButton)

$validateButton = New-Button -Text "Validate" -X 24 -Y 172 -Width 130 -Height 38
$evaluateButton = New-Button -Text "Evaluate" -X 166 -Y 172 -Width 130 -Height 38
$suggestButton = New-Button -Text "Preview Events" -X 308 -Y 172 -Width 140 -Height 38
$runButton = New-Button -Text "Write Events" -X 460 -Y 172 -Width 140 -Height 38
$mainPanel.Controls.Add($validateButton)
$mainPanel.Controls.Add($evaluateButton)
$mainPanel.Controls.Add($suggestButton)
$mainPanel.Controls.Add($runButton)

$disabledCapture = New-Button -Text "Live Capture (Next)" -X 630 -Y 172 -Width 160 -Height 38
$disabledCapture.Enabled = $false
$mainPanel.Controls.Add($disabledCapture)

$disabledOverlay = New-Button -Text "Overlay (Next)" -X 802 -Y 172 -Width 130 -Height 38
$disabledOverlay.Enabled = $false
$mainPanel.Controls.Add($disabledOverlay)

$metricsPanel = New-Object System.Windows.Forms.Panel
$metricsPanel.Location = New-Object System.Drawing.Point -ArgumentList 24, 232
$metricsPanel.Size = New-Object System.Drawing.Size -ArgumentList 300, 230
$metricsPanel.Anchor = "Bottom,Left"
$metricsPanel.BackColor = $Colors.PanelAlt
$mainPanel.Controls.Add($metricsPanel)

$metricsPanel.Controls.Add((New-Label -Text "Metrics" -X 18 -Y 16 -Width 220 -Height 24 -Size 12 -Style ([System.Drawing.FontStyle]::Bold)))
$framesMetric = New-Label -Text "frames: -" -X 18 -Y 54 -Width 250 -Height 22 -Color $Colors.Muted
$objectsMetric = New-Label -Text "objects: -" -X 18 -Y 82 -Width 250 -Height 22 -Color $Colors.Muted
$targetsMetric = New-Label -Text "targets: -" -X 18 -Y 110 -Width 250 -Height 22 -Color $Colors.Muted
$confidenceMetric = New-Label -Text "mean confidence: -" -X 18 -Y 138 -Width 250 -Height 22 -Color $Colors.Muted
$distanceMetric = New-Label -Text "mean distance: -" -X 18 -Y 166 -Width 250 -Height 22 -Color $Colors.Muted
$metricsPanel.Controls.Add($framesMetric)
$metricsPanel.Controls.Add($objectsMetric)
$metricsPanel.Controls.Add($targetsMetric)
$metricsPanel.Controls.Add($confidenceMetric)
$metricsPanel.Controls.Add($distanceMetric)

$outputBox = New-TextBox -X 344 -Y 232 -Width 588 -Height 230 -Multiline
$outputBox.Anchor = "Top,Bottom,Left,Right"
$mainPanel.Controls.Add($outputBox)

$statusLabel = New-Label -Text "Ready." -X 24 -Y 642 -Width 920 -Height 24 -Size 9 -Color $Colors.Muted
$statusLabel.Anchor = "Bottom,Left,Right"
$form.Controls.Add($statusLabel)

$selectButton.Add_Click({
    $dialog = New-Object System.Windows.Forms.OpenFileDialog
    $dialog.Filter = "JSONL files (*.jsonl)|*.jsonl|All files (*.*)|*.*"
    $dialog.Title = "Select frame JSONL"
    if ($dialog.ShowDialog($form) -eq [System.Windows.Forms.DialogResult]::OK) {
        $inputBox.Text = $dialog.FileName
        Set-Status "Selected $($dialog.FileName)"
    }
})

$sampleButton.Add_Click({
    if (Test-Path $SampleFile -PathType Leaf) {
        $inputBox.Text = $SampleFile
        Set-Status "Loaded bundled sample file."
    }
    else {
        [System.Windows.Forms.MessageBox]::Show($form, "Sample file is not included in this package.", "AutoAim Review")
    }
})

$outputBrowseButton.Add_Click({
    $dialog = New-Object System.Windows.Forms.SaveFileDialog
    $dialog.Filter = "JSONL files (*.jsonl)|*.jsonl|All files (*.*)|*.*"
    $dialog.Title = "Save inference event JSONL"
    $dialog.FileName = "events.jsonl"
    if ($dialog.ShowDialog($form) -eq [System.Windows.Forms.DialogResult]::OK) {
        $outputPathBox.Text = $dialog.FileName
        Set-Status "Output path set."
    }
})

$openFolderButton.Add_Click({
    $folder = Split-Path -Parent $outputPathBox.Text.Trim()
    if ($folder -and (Test-Path $folder)) {
        Start-Process explorer.exe $folder
    }
})

$validateButton.Add_Click({
    try {
        $path = Assert-InputFile
        Set-Status "Validating..."
        $result = Invoke-AutoAim -Arguments @("validate", $path)
        Append-Log $result.Command
        Append-Log $result.Stdout
        if ($result.Stderr) { Append-Log $result.Stderr }
        if ($result.ExitCode -ne 0) { throw "Validation failed." }
        Set-Status "Validation complete."
    }
    catch {
        Set-Status $_.Exception.Message
        Append-Log $_.Exception.Message
    }
})

$evaluateButton.Add_Click({
    try {
        $path = Assert-InputFile
        Set-Status "Evaluating..."
        $result = Invoke-AutoAim -Arguments @("evaluate", $path, "--json")
        Append-Log $result.Command
        Append-Log $result.Stdout
        if ($result.Stderr) { Append-Log $result.Stderr }
        if ($result.ExitCode -ne 0) { throw "Evaluation failed." }

        $summary = $result.Stdout | ConvertFrom-Json
        $framesMetric.Text = "frames: $($summary.frame_count)"
        $objectsMetric.Text = "objects: $($summary.object_count)"
        $targetsMetric.Text = "targets: $($summary.target_count)"
        $confidenceMetric.Text = "mean confidence: {0:N4}" -f [double]$summary.mean_confidence
        $distanceMetric.Text = "mean distance: {0:N2}" -f [double]$summary.mean_distance
        Set-Status "Evaluation complete."
    }
    catch {
        Set-Status $_.Exception.Message
        Append-Log $_.Exception.Message
    }
})

$suggestButton.Add_Click({
    try {
        $path = Assert-InputFile
        Set-Status "Generating preview events..."
        $result = Invoke-AutoAim -Arguments @("suggest", $path)
        Append-Log $result.Command
        Append-Log $result.Stdout
        if ($result.Stderr) { Append-Log $result.Stderr }
        if ($result.ExitCode -ne 0) { throw "Suggestion generation failed." }
        Set-Status "Preview events generated."
    }
    catch {
        Set-Status $_.Exception.Message
        Append-Log $_.Exception.Message
    }
})

$runButton.Add_Click({
    try {
        $path = Assert-InputFile
        $outputPath = $outputPathBox.Text.Trim()
        if ([string]::IsNullOrWhiteSpace($outputPath)) {
            throw "Choose an output JSONL path first."
        }

        $parent = Split-Path -Parent $outputPath
        if ($parent) {
            New-Item -ItemType Directory -Path $parent -Force | Out-Null
        }

        Set-Status "Writing runtime events..."
        $result = Invoke-AutoAim -Arguments @("run-jsonl", $path, $outputPath)
        Append-Log $result.Command
        Append-Log $result.Stdout
        if ($result.Stderr) { Append-Log $result.Stderr }
        if ($result.ExitCode -ne 0) { throw "Runtime event generation failed." }
        Set-Status "Wrote events to $outputPath"
    }
    catch {
        Set-Status $_.Exception.Message
        Append-Log $_.Exception.Message
    }
})

if (-not (Test-Path $AutoAimExe -PathType Leaf)) {
    Set-Status "Missing CLI executable: $AutoAimExe"
    Append-Log "This GUI requires bin\autoaim.exe from the Windows package."
}

[void]$form.ShowDialog()
