; Inno Setup script for Circuit Simulator
; Compiles the staged directory (dist\windows) produced by package.ps1
; Build with: iscc scripts\windows\circuitsimulator.iss

#define MyAppName "Circuit Simulator"
#ifndef MyAppVersion
  #define CargoToml AddBackslash(SourcePath) + "..\..\Cargo.toml"
  #define MyAppVersion StringChange(ReadIni(CargoToml, "workspace.package", "version", "0.1.0"), '"', '')
#endif
#define MyAppPublisher "Veli Tasalı"
#define MyAppURL "https://github.com/velitasali/CircuitSimulator"
#define MyAppExeName "circuitsimulator.exe"

[Setup]
AppId={{E5D81347-BD65-4C8D-9134-45300B7EFB51}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}/issues
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
OutputDir=..\..\dist
OutputBaseFilename=CircuitSimulator-setup
SetupIconFile=..\..\resources\icons\circuitsimulator.ico
UninstallDisplayIcon={app}\circuitsimulator.ico
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ChangesAssociations=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "fileassoc"; Description: "Associate .circ1 files with Circuit Simulator"; GroupDescription: "File associations:"

[InstallDelete]
Type: files; Name: "{app}\D3Dcompiler_47.dll"
Type: files; Name: "{app}\opengl32sw.dll"

[Files]
Source: "..\..\dist\windows\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\..\resources\icons\circuitsimulator.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\circuitsimulator.ico"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"; IconFilename: "{app}\circuitsimulator.ico"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\circuitsimulator.ico"; Tasks: desktopicon

[Registry]
Root: HKA; Subkey: "Software\Classes\.circ1"; ValueType: string; ValueName: ""; ValueData: "CircuitSimulator.Workspace"; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\CircuitSimulator.Workspace"; ValueType: string; ValueName: ""; ValueData: "Circuit Simulator Workspace"; Flags: uninsdeletekey; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\CircuitSimulator.Workspace\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\circuitsimulator.ico"; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\CircuitSimulator.Workspace\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: fileassoc

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
