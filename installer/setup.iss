; GiangEd Attendance - Inno Setup Script
; Creates Windows installer with database configuration

#define AppName "GiangEd Attendance"
#define AppVersion "0.1.0"
#define AppPublisher "Gianged"
#define AppExeName "gianged-attendance.exe"

[Setup]
AppId={{F7B3D5A1-8E2C-4F6A-9D1B-3C5E7F9A2B4D}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
OutputDir=..\target\installer
OutputBaseFilename=GiangEd-Attendance-Setup-{#AppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
Source: "..\target\release\{#AppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\config.example.toml"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\database.sql"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"
Name: "{group}\Uninstall {#AppName}"; Filename: "{uninstallexe}"
Name: "{userdesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional icons:"

[Run]
Filename: "{app}\{#AppExeName}"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent

[Code]
var
  DatabasePage: TInputQueryWizardPage;

procedure InitializeWizard;
begin
  // Create custom page for database configuration
  DatabasePage := CreateInputQueryPage(wpSelectDir,
    'Database Configuration',
    'Configure PostgreSQL Connection',
    'Enter the database connection details. You can change these later in the application settings.');

  DatabasePage.Add('Host:', False);
  DatabasePage.Add('Port:', False);
  DatabasePage.Add('Database Name:', False);
  DatabasePage.Add('Username:', False);
  DatabasePage.Add('Password:', True);

  // Set default values
  DatabasePage.Values[0] := 'localhost';
  DatabasePage.Values[1] := '5432';
  DatabasePage.Values[2] := 'gianged_attendance';
  DatabasePage.Values[3] := 'postgres';
  DatabasePage.Values[4] := '';
end;

function NextButtonClick(CurPageID: Integer): Boolean;
begin
  Result := True;

  if CurPageID = DatabasePage.ID then
  begin
    // Validate required fields
    if Trim(DatabasePage.Values[0]) = '' then
    begin
      MsgBox('Database host is required.', mbError, MB_OK);
      Result := False;
      Exit;
    end;

    if Trim(DatabasePage.Values[1]) = '' then
    begin
      MsgBox('Database port is required.', mbError, MB_OK);
      Result := False;
      Exit;
    end;

    if Trim(DatabasePage.Values[2]) = '' then
    begin
      MsgBox('Database name is required.', mbError, MB_OK);
      Result := False;
      Exit;
    end;

    if Trim(DatabasePage.Values[3]) = '' then
    begin
      MsgBox('Database username is required.', mbError, MB_OK);
      Result := False;
      Exit;
    end;
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  ConfigContent: String;
  ConfigPath: String;
begin
  if CurStep = ssPostInstall then
  begin
    // Generate config.toml with user-provided values
    ConfigContent :=
      '[device]' + #13#10 +
      'url = "http://192.168.90.11"' + #13#10 +
      'username = "administrator"' + #13#10 +
      'password = ""' + #13#10 +
      #13#10 +
      '[database]' + #13#10 +
      'host = "' + DatabasePage.Values[0] + '"' + #13#10 +
      'port = ' + DatabasePage.Values[1] + #13#10 +
      'name = "' + DatabasePage.Values[2] + '"' + #13#10 +
      'username = "' + DatabasePage.Values[3] + '"' + #13#10 +
      'password = "' + DatabasePage.Values[4] + '"' + #13#10 +
      #13#10 +
      '[sync]' + #13#10 +
      'days = 30' + #13#10 +
      'max_user_id = 300' + #13#10 +
      'auto_enabled = false' + #13#10 +
      'interval_minutes = 60' + #13#10 +
      #13#10 +
      '[ui]' + #13#10 +
      'start_minimized = false' + #13#10 +
      'minimize_to_tray = true' + #13#10;

    ConfigPath := ExpandConstant('{app}\config.toml');
    SaveStringToFile(ConfigPath, ConfigContent, False);
  end;
end;
