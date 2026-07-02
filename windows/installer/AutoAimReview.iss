#ifndef AppVersion
#define AppVersion "0.0.0-dev"
#endif

#ifndef SourceDir
#define SourceDir "..\..\dist\windows\package-root"
#endif

#ifndef OutputDir
#define OutputDir "..\..\dist\windows"
#endif

#define AppName "AutoAim Review"
#define AppPublisher "ccctw-ma"
#define AppUrl "https://github.com/ccctw-ma/AutoAim"
#define AppExeName "AutoAimReview.exe"

[Setup]
AppId={{7A89D7B1-F83C-4B35-AF10-211D5C8912D6}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppUrl}
AppSupportURL={#AppUrl}
AppUpdatesURL={#AppUrl}
DefaultDirName={localappdata}\AutoAimReview
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputDir={#OutputDir}
OutputBaseFilename=AutoAimReviewSetup-x64
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
SetupIconFile={#SourceDir}\assets\logo.ico
UninstallDisplayIcon={app}\assets\logo.ico
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"; Flags: checkedonce

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\AutoAim Review"; Filename: "{app}\{#AppExeName}"; WorkingDir: "{app}"; IconFilename: "{app}\assets\logo.ico"
Name: "{group}\AutoAim CLI"; Filename: "{app}\bin\autoaim.exe"; WorkingDir: "{app}"; IconFilename: "{app}\assets\logo.ico"
Name: "{autodesktop}\AutoAim Review"; Filename: "{app}\{#AppExeName}"; WorkingDir: "{app}"; IconFilename: "{app}\assets\logo.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExeName}"; Description: "Launch AutoAim Review"; Flags: postinstall nowait skipifsilent unchecked
