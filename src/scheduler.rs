use crate::{capture, ocr, storage::Database};
use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tokio::signal;

pub async fn run(db: Database, db_path: &Path, interval_secs: u64, cleanup_days: Option<u64>) -> Result<()> {
    let pid = std::process::id();
    let pid_path = db_path.with_extension("pid");
    std::fs::write(&pid_path, pid.to_string())?;

    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
    let mut last_cleanup = chrono::Local::now();

    loop {
        tokio::select! {
            _ = interval.tick() => {
                match capture_screen_once(&db).await {
                    Ok(id) => println!("[{}] Screenshot #{} gespeichert.", chrono::Local::now().format("%H:%M:%S"), id),
                    Err(e) => eprintln!("[{}] Fehler: {}", chrono::Local::now().format("%H:%M:%S"), e),
                }

                if let Some(days) = cleanup_days {
                    if last_cleanup + chrono::Duration::hours(24) <= chrono::Local::now() {
                        match db.cleanup(days) {
                            Ok(n) if n > 0 => println!("[{}] Aufgeraeumt: {} alte Screenshots geloescht.", chrono::Local::now().format("%H:%M:%S"), n),
                            Err(e) => eprintln!("[{}] Cleanup-Fehler: {}", chrono::Local::now().format("%H:%M:%S"), e),
                            _ => {}
                        }
                        last_cleanup = chrono::Local::now();
                    }
                }
            }
            _ = signal::ctrl_c() => {
                println!("\nRecall wird beendet.");
                let _ = std::fs::remove_file(&pid_path);
                break;
            }
        }
    }

    Ok(())
}

async fn capture_screen_once(db: &Database) -> Result<i64> {
    let data = capture::capture_screen()?;
    let ocr_text = ocr::recognize(&data)?;
    let id = db.save_screenshot(&data, &ocr_text)?;
    Ok(id)
}

const TASK_PREFIX: &str = "Recall";

fn task_name_start(name: &str) -> String {
    format!("{}_{}", TASK_PREFIX, name)
}

fn task_name_stop(name: &str) -> String {
    format!("{}_{}_stop", TASK_PREFIX, name)
}

fn day_code(day: &str) -> Result<String> {
    match day.to_uppercase().as_str() {
        "MON" | "MONDAY" => Ok("Monday".into()),
        "TUE" | "TUESDAY" => Ok("Tuesday".into()),
        "WED" | "WEDNESDAY" => Ok("Wednesday".into()),
        "THU" | "THURSDAY" => Ok("Thursday".into()),
        "FRI" | "FRIDAY" => Ok("Friday".into()),
        "SAT" | "SATURDAY" => Ok("Saturday".into()),
        "SUN" | "SUNDAY" => Ok("Sunday".into()),
        _ => anyhow::bail!("Unbekannter Wochentag: {} (verwende MON-SUN)", day),
    }
}

fn ps_hours_minutes(time: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = time.split(':').collect();
    if parts.len() != 2 {
        anyhow::bail!("Ungueltiges Zeitformat: {} (erwartet HH:MM)", time);
    }
    let hours: u32 = parts[0].parse().context("Ungueltige Stunden")?;
    let minutes: u32 = parts[1].parse().context("Ungueltige Minuten")?;
    Ok((hours.to_string(), minutes.to_string()))
}

pub fn schedule_task(name: &str, day: &str, start: &str, end: &str, interval: u64) -> Result<()> {
    let exe = std::env::current_exe()
        .context("Kann Pfad der Binary nicht ermitteln")?;
    let exe_str = exe.to_string_lossy().to_string();
    let day_name = day_code(day)?;
    let (start_h, start_m) = ps_hours_minutes(start)?;
    let (end_h, end_m) = ps_hours_minutes(end)?;

    let start_name = task_name_start(name);
    let stop_name = task_name_stop(name);

    let recall_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".recall");
    std::fs::create_dir_all(&recall_dir)?;

    let ps1_path = recall_dir.join("schedule_task.ps1");
    let start_vbs = recall_dir.join("recall_start.vbs");
    let stop_vbs = recall_dir.join("recall_stop.vbs");

    let start_cmd = if interval < 60 {
        format!("\"{}\" start --interval {}", exe_str, interval)
    } else {
        format!("\"{}\" snapshot", exe_str)
    };

    let start_vbs_content = format!(
        "Set WshShell = CreateObject(\"WScript.Shell\")\r\nWshShell.Run \"{}\", 0, False\r\n",
        start_cmd.replace('"', "\"\"")
    );
    std::fs::write(&start_vbs, start_vbs_content)?;

    let stop_vbs_content = format!(
        "Set WshShell = CreateObject(\"WScript.Shell\")\r\nWshShell.Run \"taskkill /F /IM recall-cli.exe\", 0, False\r\n"
    );
    std::fs::write(&stop_vbs, stop_vbs_content)?;

    let ps_script = if interval < 60 {
        format!(
            r#"param(
    [string]$StartVbs,
    [string]$StopVbs,
    [string]$StartTask,
    [string]$StopTask,
    [string]$Day,
    [int]$StartH,
    [int]$StartM,
    [int]$EndH,
    [int]$EndM
)

$startTime = Get-Date -Hour $StartH -Minute $StartM -Second 0
$endTime = Get-Date -Hour $EndH -Minute $EndM -Second 0

$trigger = New-ScheduledTaskTrigger -Weekly -DaysOfWeek $Day -At $startTime
$action = New-ScheduledTaskAction -Execute 'wscript.exe' -Argument "`"$StartVbs`""
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable

Register-ScheduledTask -TaskName $StartTask -Trigger $trigger -Action $action -Settings $settings -Force | Out-Null
Write-Host "Start-Task '$StartTask' erstellt."

$stopTrigger = New-ScheduledTaskTrigger -Weekly -DaysOfWeek $Day -At $endTime
$stopAction = New-ScheduledTaskAction -Execute 'wscript.exe' -Argument "`"$StopVbs`""
Register-ScheduledTask -TaskName $StopTask -Trigger $stopTrigger -Action $stopAction -Settings $settings -Force | Out-Null
Write-Host "Stop-Task '$StopTask' erstellt."
"#)
    } else {
        format!(
            r#"param(
    [string]$StartVbs,
    [string]$StopVbs,
    [string]$StartTask,
    [string]$StopTask,
    [string]$Day,
    [int]$StartH,
    [int]$StartM,
    [int]$EndH,
    [int]$EndM,
    [int]$IntervalMin
)

$startTime = Get-Date -Hour $StartH -Minute $StartM -Second 0
$endTime = Get-Date -Hour $EndH -Minute $EndM -Second 0
$duration = $endTime - $startTime

$trigger = New-ScheduledTaskTrigger -Weekly -DaysOfWeek $Day -At $startTime
$trigger.Repetition = (New-ScheduledTaskTrigger -Once -At $startTime `
    -RepetitionInterval (New-TimeSpan -Minutes $IntervalMin) `
    -RepetitionDuration $duration).Repetition

$action = New-ScheduledTaskAction -Execute 'wscript.exe' -Argument "`"$StartVbs`""
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable

Register-ScheduledTask -TaskName $StartTask -Trigger $trigger -Action $action -Settings $settings -Force | Out-Null
Write-Host "Start-Task '$StartTask' erstellt."

$stopTrigger = New-ScheduledTaskTrigger -Weekly -DaysOfWeek $Day -At $endTime
$stopAction = New-ScheduledTaskAction -Execute 'wscript.exe' -Argument "`"$StopVbs`""
Register-ScheduledTask -TaskName $StopTask -Trigger $stopTrigger -Action $stopAction -Settings $settings -Force | Out-Null
Write-Host "Stop-Task '$StopTask' erstellt."
"#)
    };

    std::fs::write(&ps1_path, &ps_script)?;

    let interval_min = (interval / 60).to_string();

    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy", "Bypass",
            "-File", &ps1_path.to_string_lossy(),
            "-StartVbs", &start_vbs.to_string_lossy(),
            "-StopVbs", &stop_vbs.to_string_lossy(),
            "-StartTask", &start_name,
            "-StopTask", &stop_name,
            "-Day", &day_name,
            "-StartH", &start_h,
            "-StartM", &start_m,
            "-EndH", &end_h,
            "-EndM", &end_m,
            "-IntervalMin", &interval_min,
        ])
        .output()
        .context("Fehler beim Ausfuehren von PowerShell")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!("PowerShell Fehler:\n{}\n{}", stdout, stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    print!("{}", stdout);

    println!("\nTask-Scheduler Eintraege erstellt (unsichtbar):");
    println!("  {} - Jeden {} von {} bis {}, alle {}s", start_name, day, start, end, interval);
    println!("  {} - Stoppt den Prozess um {}", stop_name, end);
    println!("\nVerwende 'recall-cli schedule-list' zum Anzeigen der Tasks.");

    Ok(())
}

pub fn unschedule_task(name: &str) -> Result<()> {
    let start_name = task_name_start(name);
    let stop_name = task_name_stop(name);

    let recall_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".recall");
    let ps1_path = recall_dir.join("unschedule_task.ps1");

    let ps_script = format!(
        r#"param(
    [string]$StartTask,
    [string]$StopTask
)

$tasks = @($StartTask, $StopTask)
foreach ($t in $tasks) {{
    $existing = Get-ScheduledTask -TaskName $t -ErrorAction SilentlyContinue
    if ($existing) {{
        Unregister-ScheduledTask -TaskName $t -Confirm:$false
        Write-Host "Task '$t' entfernt."
    }} else {{
        Write-Host "Task '$t' nicht gefunden."
    }}
}}
"#);

    std::fs::write(&ps1_path, &ps_script)?;

    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy", "Bypass",
            "-File", &ps1_path.to_string_lossy(),
            "-StartTask", &start_name,
            "-StopTask", &stop_name,
        ])
        .output()
        .context("Fehler beim Ausfuehren von PowerShell")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("PowerShell Fehler: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    print!("{}", stdout);

    Ok(())
}

pub fn list_tasks() -> Result<()> {
    let recall_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".recall");
    let ps1_path = recall_dir.join("list_tasks.ps1");

    let ps_script = r#"param(
    [string]$Prefix
)

$tasks = Get-ScheduledTask -TaskName "$Prefix*" -ErrorAction SilentlyContinue
if (-not $tasks) {
$tasks = Get-ScheduledTask -TaskName "$Prefix*" -ErrorAction SilentlyContinue
}
if (-not $tasks) {
    Write-Host "Keine Recall-Tasks gefunden."
} else {
    Write-Host "Recall Task-Scheduler Eintraege:"
    Write-Host ("-" * 80)
    foreach ($task in $tasks) {
        $triggers = $task.Triggers
        $triggerInfo = ""
        foreach ($tr in $triggers) {
            $className = $tr.CimClass.CimClassName
            if ($className -match 'MSFT_TaskWeeklyTrigger') {
                $days = ""
                if ($tr.DaysOfWeek -band 1) { $days += "Son, " }
                if ($tr.DaysOfWeek -band 2) { $days += "Mon, " }
                if ($tr.DaysOfWeek -band 4) { $days += "Die, " }
                if ($tr.DaysOfWeek -band 8) { $days += "Mit, " }
                if ($tr.DaysOfWeek -band 16) { $days += "Don, " }
                if ($tr.DaysOfWeek -band 32) { $days += "Fre, " }
                if ($tr.DaysOfWeek -band 64) { $days += "Sam, " }
                $days = $days.TrimEnd(', ')
                $triggerInfo = "Weekly ($days)"
                if ($tr.Repetition.Interval) {
                    $triggerInfo += " | Intervall: $($tr.Repetition.Interval) bis $($tr.Repetition.Duration)"
                }
            } elseif ($className -match 'MSFT_TaskTimeTrigger') {
                $triggerInfo = "Einmalig"
            }
        }
        $actionInfo = ""
        foreach ($act in $task.Actions) {
            $actionInfo = "$($act.Execute) $($act.Arguments)"
        }
        Write-Host "  $($task.TaskName)"
        Write-Host "    Status: $($task.State)"
        Write-Host "    Trigger: $triggerInfo"
        Write-Host "    Aktion: $actionInfo"
        Write-Host ""
    }
    Write-Host ("-" * 80)
    Write-Host "Gesamt: $($tasks.Count) Task(s)"
}
"#;

    std::fs::write(&ps1_path, ps_script)?;

    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy", "Bypass",
            "-File", &ps1_path.to_string_lossy(),
            "-Prefix", TASK_PREFIX,
        ])
        .output()
        .context("Fehler beim Ausfuehren von PowerShell")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("PowerShell Fehler: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    print!("{}", stdout);

    Ok(())
}
