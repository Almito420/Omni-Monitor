Set sh = CreateObject("WScript.Shell")
sh.Run """" & Left(WScript.ScriptFullName, InStrRev(WScript.ScriptFullName, "\")) & "omni_monitor.exe""", 0, False
