#define MyAppName "DSH-X"
#ifndef MyAppVersion
#define MyAppVersion "0.1.11"
#endif
#define MyAppPublisher "yyh"
#define MyAppExeName "DSH.exe"
#ifndef MyAppIcon
#define MyAppIcon "dsh.ico"
#endif

[Setup]
AppId={{8F3C2A91-6B47-4E1D-9C5A-2D8E0F4B7A16}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={localappdata}\Programs\DSH
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=..\release
OutputBaseFilename=DSH-Setup
SetupIconFile=..\assets\dsh.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
UsePreviousAppDir=yes
CloseApplications=yes
RestartApplications=no
AllowNoIcons=yes

[Tasks]
; 不带 checkedonce 就是默认勾选；带上它反而变成「首次装默认不勾」，正是之前的行为
Name: "desktopicon"; Description: "创建桌面快捷方式"; GroupDescription: "附加任务:"

[Files]
Source: "..\release\DSH\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"; IconFilename: "{app}\assets\{#MyAppIcon}"; Tasks: desktopicon
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"; IconFilename: "{app}\assets\{#MyAppIcon}"
Name: "{group}\卸载 {#MyAppName}"; Filename: "{uninstallexe}"

[Languages]
; 后者覆盖前者：中文包补全全部消息，Default.isl 在前只作兜底
Name: "chinesesimplified"; MessagesFile: "compiler:Default.isl,ChineseSimplified.isl"

[Run]
Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"; Description: "安装完成后启动"; Flags: nowait postinstall skipifsilent shellexec runasoriginaluser
; 完成页的「删除安装包」勾选框，默认选中：多数人装完不会留着安装包，留下来只是占地方。
; 不想删的在完成页取消勾选即可（静默安装时整条跳过，不会不打招呼地删文件）。
; 之所以拿 ping 拖三秒：此刻 setup.exe 自己还在运行，直接 del 会被拒绝；cmd 独立于安装
; 程序存活，等它退出之后再删就干净了。
Filename: "{cmd}"; Parameters: "/c ping -n 3 127.0.0.1 > nul & del /f /q ""{srcexe}"""; Description: "删除安装包"; Flags: runhidden nowait postinstall skipifsilent

[UninstallDelete]
Type: filesandordirs; Name: "{app}\node_modules"
