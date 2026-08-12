#ifndef MyAppVersion
  #error MyAppVersion must be provided with /DMyAppVersion=<version>
#endif

#ifndef SourceDir
  #error SourceDir must be provided with /DSourceDir=<absolute path>
#endif

#ifndef WebView2Bootstrapper
  #error WebView2Bootstrapper must be provided with /DWebView2Bootstrapper=<absolute path>
#endif

[Setup]
AppId={{D4969F5C-8921-4C4D-8892-8D2A60BF2EAB}
AppName=Asterline Desktop
AppVersion={#MyAppVersion}
AppPublisher=Asterline contributors
AppPublisherURL=https://github.com/song0705/Asterline
AppSupportURL=https://github.com/song0705/Asterline/issues
AppUpdatesURL=https://github.com/song0705/Asterline/releases
DefaultDirName={localappdata}\Programs\Asterline Desktop
DefaultGroupName=Asterline Desktop
DisableProgramGroupPage=yes
LicenseFile=..\..\LICENSE
OutputDir=..\..\dist
OutputBaseFilename=asterline-desktop-{#MyAppVersion}-x86_64-windows-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
CloseApplications=yes
RestartApplications=no
UninstallDisplayName=Asterline Desktop
UninstallDisplayIcon={app}\asterline-desktop.exe

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"; Flags: unchecked

[Files]
Source: "{#WebView2Bootstrapper}"; DestName: "MicrosoftEdgeWebview2Setup.exe"; Flags: dontcopy noencryption
Source: "{#SourceDir}\asterline-desktop.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Asterline Desktop"; Filename: "{app}\asterline-desktop.exe"; WorkingDir: "{app}"
Name: "{autodesktop}\Asterline Desktop"; Filename: "{app}\asterline-desktop.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Run]
Filename: "{app}\asterline-desktop.exe"; Description: "Launch Asterline Desktop"; Flags: nowait postinstall skipifsilent

[Code]
const
  WebView2ClientKey = 'Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}';

function HasWebView2Version(RootKey: Integer): Boolean;
var
  Version: String;
begin
  Result := RegQueryStringValue(RootKey, WebView2ClientKey, 'pv', Version) and
    (Version <> '') and (CompareText(Version, '0.0.0.0') <> 0);
end;

function IsWebView2RuntimeInstalled: Boolean;
begin
  Result := HasWebView2Version(HKLM32) or HasWebView2Version(HKCU);
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ResultCode: Integer;
  WaitPid: Integer;
begin
  Result := '';
  WaitPid := StrToIntDef(ExpandConstant('{param:WAITPID|0}'), 0);
  if WaitPid > 0 then
    Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'),
      '-NoLogo -NoProfile -NonInteractive -Command "Wait-Process -Id ' +
      IntToStr(WaitPid) + ' -ErrorAction SilentlyContinue"', '', SW_HIDE,
      ewWaitUntilTerminated, ResultCode);

  if not IsWebView2RuntimeInstalled then
  begin
    ExtractTemporaryFile('MicrosoftEdgeWebview2Setup.exe');
    if not Exec(ExpandConstant('{tmp}\MicrosoftEdgeWebview2Setup.exe'),
      '/silent /install', '', SW_HIDE, ewWaitUntilTerminated, ResultCode) then
    begin
      Result := 'Microsoft Edge WebView2 Runtime could not be started.';
      Exit;
    end;
    if ResultCode <> 0 then
    begin
      Result := 'Microsoft Edge WebView2 Runtime installation failed with exit code ' +
        IntToStr(ResultCode) + '.';
      Exit;
    end;
    if not IsWebView2RuntimeInstalled then
      Result := 'Microsoft Edge WebView2 Runtime installation did not complete.';
  end;
end;
