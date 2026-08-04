#ifndef MyAppVersion
  #define MyAppVersion "0.0.0"
#endif

#ifndef MyAppExeSource
  #define MyAppExeSource "..\target\x86_64-pc-windows-msvc\release\pinkdown.exe"
#endif

#define MyAppId "{{6E2480A1-33C8-4BF3-A899-7229B36F2048}"
#define MyAppName "PinkDown"
#define MyAppExeName "pinkdown.exe"
#define MarkdownProgId "PinkDown.Markdown"

[Setup]
AppId={#MyAppId}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher=PinkDown
AppPublisherURL=https://github.com/3xian/PinkDown
AppSupportURL=https://github.com/3xian/PinkDown/issues
DefaultDirName={localappdata}\Programs\PinkDown
DefaultGroupName=PinkDown
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
ChangesAssociations=yes
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
OutputDir=..\dist
OutputBaseFilename=pinkdown-windows-x64-setup
UninstallDisplayIcon={app}\{#MyAppExeName}
CloseApplications=yes
RestartApplications=yes

[Files]
Source: "{#MyAppExeSource}"; DestDir: "{app}"; DestName: "{#MyAppExeName}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\PinkDown"; Filename: "{app}\{#MyAppExeName}"

[Registry]
Root: HKA; Subkey: "Software\Classes\.md"; ValueType: string; ValueName: ""; ValueData: "{#MarkdownProgId}"; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.md\OpenWithProgids"; ValueType: string; ValueName: "{#MarkdownProgId}"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\{#MarkdownProgId}"; ValueType: string; ValueName: ""; ValueData: "Markdown Document"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\{#MarkdownProgId}\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"
Root: HKA; Subkey: "Software\Classes\{#MarkdownProgId}\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""
Root: HKA; Subkey: "Software\PinkDown\Capabilities"; ValueType: string; ValueName: "ApplicationName"; ValueData: "PinkDown"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\PinkDown\Capabilities"; ValueType: string; ValueName: "ApplicationDescription"; ValueData: "Native Markdown editor and reader"
Root: HKA; Subkey: "Software\PinkDown\Capabilities\FileAssociations"; ValueType: string; ValueName: ".md"; ValueData: "{#MarkdownProgId}"
Root: HKA; Subkey: "Software\RegisteredApplications"; ValueType: string; ValueName: "PinkDown"; ValueData: "Software\PinkDown\Capabilities"; Flags: uninsdeletevalue

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Launch PinkDown"; Flags: nowait postinstall skipifsilent
