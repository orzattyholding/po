; ══════════════════════════════════════════════════════════════
; Protocol Orzatty (PO) — Windows Installer
; Built with Inno Setup (https://jrsoftware.org/isinfo.php)
;
; To compile this into a wizard .exe:
;   1. Download Inno Setup from https://jrsoftware.org/isdl.php
;   2. Open this file (installer.iss) in Inno Setup
;   3. Click Build > Compile (or Ctrl+F9)
;   4. Output: PO\dist\ProtocolOrzatty-Setup.exe
; ══════════════════════════════════════════════════════════════

#define MyAppName "Protocol Orzatty"
#define MyAppShortName "po"
#define MyAppVersion "0.1.0"
#define MyAppPublisher "Orzatty"
#define MyAppURL "https://orzatty.com"
#define MyAppExeName "po.exe"

[Setup]
AppId={{E4A2F8C1-7D3B-4A5E-9F1C-8B2D6E0A3F47}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
DefaultDirName={userpf}\{#MyAppName}
DefaultGroupName={#MyAppName}
LicenseFile=LICENSE
OutputDir=dist
OutputBaseFilename=ProtocolOrzatty-{#MyAppVersion}-Setup
Compression=lzma2/ultra64
SolidCompression=yes
PrivilegesRequired=lowest
ChangesEnvironment=yes
WizardStyle=modern
; SetupIconFile=assets\po-icon.ico  ; Uncomment when icon is available
UninstallDisplayIcon={app}\{#MyAppExeName}
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

; Modern look
; WizardImageFile=assets\wizard-banner.bmp       ; Uncomment when assets are available
; WizardSmallImageFile=assets\wizard-icon.bmp     ; Uncomment when assets are available

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"

[Messages]
english.WelcomeLabel2=This will install [name/ver] on your computer.%n%nProtocol Orzatty is an end-to-end encrypted peer-to-peer communication protocol built on QUIC/UDP.%n%nEvery byte on the wire is encrypted. There is no plaintext mode.
spanish.WelcomeLabel2=Esto instalara [name/ver] en tu computadora.%n%nProtocol Orzatty es un protocolo de comunicacion peer-to-peer con cifrado end-to-end sobre QUIC/UDP.%n%nCada byte en la red esta cifrado. No existe modo texto plano.

[Tasks]
Name: "addtopath"; Description: "Add 'po' to system PATH (recommended)"; GroupDescription: "Environment:"; Flags: checkedonce

[Files]
; The compiled CLI binary — renamed from po-cli.exe to po.exe
Source: "target\release\po-cli.exe"; DestDir: "{app}"; DestName: "po.exe"; Flags: ignoreversion
; Documentation
Source: "README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "WHITEPAPER.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Protocol Orzatty CLI"; Filename: "{cmd}"; Parameters: "/k ""{app}\po.exe"" --help"; WorkingDir: "{app}"
Name: "{group}\Uninstall Protocol Orzatty"; Filename: "{uninstallexe}"

[Registry]
; Add to user PATH
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Tasks: addtopath; Check: NeedsAddPath(ExpandConstant('{app}'))

[Run]
Filename: "{app}\{#MyAppExeName}"; Parameters: "identity"; Flags: nowait postinstall skipifsilent runascurrentuser; Description: "Show your node identity"

[Code]
// Check if the path already contains our install directory
function NeedsAddPath(Param: string): boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OrigPath) then
  begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + Param + ';', ';' + OrigPath + ';') = 0;
end;

// Notify the system that environment variables changed
procedure CurStepChanged(CurStep: TSetupStep);
var
  ResultCode: Integer;
begin
  if CurStep = ssPostInstall then
  begin
    // Broadcast WM_SETTINGCHANGE so open terminals pick up the new PATH
    Exec('cmd.exe', '/c setx PROMPT "$P$G" >nul 2>&1', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  end;
end;
