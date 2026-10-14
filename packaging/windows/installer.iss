; Built by CI: iscc /DAppVersion=x.y.z /DSourceDir=<release dir> /DOutputDir=<dir> installer.iss
#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
#ifndef SourceDir
  #define SourceDir "..\..\target\release"
#endif
#ifndef OutputDir
  #define OutputDir "..\..\dist"
#endif

[Setup]
AppId={{6C1F4E2B-8A3D-4F5E-9B7A-2D4C6E8F0A1B}
AppName=Yarita
AppVersion={#AppVersion}
AppPublisher=technsoul
DefaultDirName={autopf}\Yarita
DefaultGroupName=Yarita
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#OutputDir}
OutputBaseFilename=Yarita-{#AppVersion}-windows-x64-setup
Compression=lzma2
SolidCompression=yes
UninstallDisplayIcon={app}\yarita.exe
SetupIconFile=..\..\crates\ui\assets\icon.ico

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "{#SourceDir}\yarita.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\yarita-service.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Yarita"; Filename: "{app}\yarita.exe"
Name: "{autodesktop}\Yarita"; Filename: "{app}\yarita.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\yarita.exe"; Description: "{cm:LaunchProgram,Yarita}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "taskkill.exe"; Parameters: "/F /IM yarita.exe"; Flags: runhidden; RunOnceId: "KillUi"
Filename: "taskkill.exe"; Parameters: "/F /IM yarita-service.exe"; Flags: runhidden; RunOnceId: "KillService"
