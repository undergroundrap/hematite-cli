#define AppName "Hematite"
#ifndef AppVersion
  #define AppVersion "0.4.4"
#endif
#ifndef BundleDir
  #define BundleDir "..\\dist\\windows\\Hematite-" + AppVersion + "-portable"
#endif
#ifndef OutputDir
  #define OutputDir "..\\dist\\windows"
#endif

[Setup]
AppId={{A1A820A0-0B8E-4B89-9BBE-5D13C90C2F67}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=Ocean Bennett
DefaultDirName={autopf}\Hematite
DefaultGroupName=Hematite
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#OutputDir}
OutputBaseFilename=Hematite-{#AppVersion}-Setup
SetupIconFile=..\assets\hematite.ico
UninstallDisplayIcon={app}\hematite.exe
Compression=lzma
SolidCompression=yes
WizardStyle=modern
ChangesEnvironment=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "addtopath"; Description: "Add Hematite to PATH"
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked

[Files]
Source: "{#BundleDir}\hematite.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BundleDir}\DirectML.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BundleDir}\README.txt"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Hematite"; Filename: "{app}\hematite.exe"; IconFilename: "{app}\hematite.exe"
Name: "{autodesktop}\Hematite"; Filename: "{app}\hematite.exe"; IconFilename: "{app}\hematite.exe"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Tasks: addtopath; Check: NeedsAddPath(ExpandConstant('{app}'))

[Code]
function NeedsAddPath(Param: string): Boolean;
var
  Paths: string;
begin
  if not RegQueryStringValue(HKCU, 'Environment', 'Path', Paths) then
    Paths := '';
  Result := Pos(';' + Uppercase(Param) + ';', ';' + Uppercase(Paths) + ';') = 0;
end;
