; Inno Setup script for "edit".
; Builds a Windows installer that installs edit.exe and registers it as an
; available "Open with" application for common text file types (the same
; approach Notepad-alternatives use — it does NOT force-replace the user's
; default app, it just makes "edit" show up as a choice, and lets the user
; set it as default from Windows' own "Open with" dialog).
;
; Build locally with Inno Setup (https://jrsoftware.org/isinfo.php):
;   ISCC.exe installer\setup.iss
; Or automatically in CI via the Minionguyjpro/Inno-Setup-Action GitHub Action
; (already wired up in .github/workflows/build.yml).

#define MyAppName "edit"
#define MyAppVersion "0.1.0"
#define MyAppExeName "edit.exe"

[Setup]
AppId={{6E3B1C2E-EDA1-4B1A-9F3E-EDIT00000001}}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
OutputBaseFilename=edit-setup-windows
Compression=lzma2
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64compatible
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
Source: "..\target\release\edit.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Создать значок на рабочем столе"; GroupDescription: "Дополнительные значки:"

; Register edit.exe as an application capable of opening text-like files,
; so it shows up under "Open with" -> "Choose another app" for any file,
; and under the listed extensions specifically. This mirrors how Notepad++
; and VS Code register themselves without silently taking over defaults.
[Registry]
Root: HKA; Subkey: "Software\Classes\Applications\{#MyAppExeName}\shell\open\command"; \
    ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\Applications\{#MyAppExeName}"; \
    ValueType: string; ValueName: "FriendlyAppName"; ValueData: "edit"; Flags: uninsdeletekey

; Advertise support for a broad set of text-like extensions (same spirit as
; Notepad's "Open any text file").
#define TextExtensions ".txt,.md,.markdown,.log,.ini,.cfg,.conf,.json,.xml,.yaml,.yml,.toml,.csv,.rs,.py,.js,.ts,.jsx,.tsx,.c,.h,.cpp,.hpp,.cs,.java,.go,.rb,.php,.sh,.bat,.ps1,.sql,.html,.htm,.css,.gitignore,.env"

[Code]
procedure RegisterExtension(const Ext: string);
begin
  RegWriteStringValue(HKA, 'Software\Classes\Applications\{#MyAppExeName}\SupportedTypes', Ext, '');
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  Extensions: TArrayOfString;
  I: Integer;
begin
  if CurStep = ssPostInstall then
  begin
    Extensions := ['.txt','.md','.markdown','.log','.ini','.cfg','.conf','.json','.xml',
                   '.yaml','.yml','.toml','.csv','.rs','.py','.js','.ts','.jsx','.tsx',
                   '.c','.h','.cpp','.hpp','.cs','.java','.go','.rb','.php','.sh','.bat',
                   '.ps1','.sql','.html','.htm','.css'];
    for I := 0 to GetArrayLength(Extensions) - 1 do
      RegisterExtension(Extensions[I]);
  end;
end;

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Запустить edit"; Flags: nowait postinstall skipifsilent
