; Inno Setup Script para onion-poc
; Compilado via GitHub Actions no Windows

#define MyAppName "Onion PoC"
#define MyAppVersion "0.6.1"
#define MyAppPublisher "Eduardo Prestes"
#define MyAppURL "https://github.com/DJmesh/onion_poc"
#define MyAppExeName "onion_poc.exe"

[Setup]
; Identificador único para a desinstalação
AppId={{E4D69B6F-311D-4C6C-874C-15FBF86C01A9}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
; Pasta padrão de instalação em Program Files
DefaultDirName={autopf}\{#MyAppName}
; Nome do grupo no Menu Iniciar
DefaultGroupName={#MyAppName}
; Mostra checkbox final "Iniciar Onion PoC"
PrivilegesRequired=admin
OutputDir=target\inno
OutputBaseFilename=onion_poc_setup_windows
Compression=lzma
SolidCompression=yes
WizardStyle=modern
; Configuração de desinstalador
UninstallDisplayIcon={app}\{#MyAppExeName}
UninstallDisplayName={#MyAppName} Uninstall

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "target\release\tor\*"; DestDir: "{app}\tor"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Registry]
; Adiciona tor.exe ao PATH do usuário
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}\tor"; Check: NeedsAddPath('{app}\tor')

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
; Garante a limpeza correta na desinstalação
Type: filesandordirs; Name: "{app}"

[Code]
function NeedsAddPath(Param: string): boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OrigPath)
  then begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + Param + ';', ';' + OrigPath + ';') = 0;
end;

