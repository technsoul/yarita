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
AppName=Focus Timer
AppVersion={#AppVersion}
AppPublisher=technsoul
DefaultDirName={autopf}\Focus Timer
DefaultGroupName=Focus Timer
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#OutputDir}
OutputBaseFilename=FocusTimer-{#AppVersion}-windows-x64-setup
Compression=lzma2
SolidCompression=yes
UninstallDisplayIcon={app}\focus-ui.exe

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "{#SourceDir}\focus-ui.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\focus-service.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Focus Timer"; Filename: "{app}\focus-ui.exe"
Name: "{autodesktop}\Focus Timer"; Filename: "{app}\focus-ui.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\focus-ui.exe"; Description: "{cm:LaunchProgram,Focus Timer}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "taskkill.exe"; Parameters: "/F /IM focus-ui.exe"; Flags: runhidden; RunOnceId: "KillUi"
Filename: "taskkill.exe"; Parameters: "/F /IM focus-service.exe"; Flags: runhidden; RunOnceId: "KillService"
