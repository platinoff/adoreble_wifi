Set fso = CreateObject("Scripting.FileSystemObject")
Set sh = CreateObject("WScript.Shell")
exe = "S:\rust\adoreble_wifi\target\debug\adoreble-api.exe"
root = "S:\rust\adoreble_wifi"
log = root & "\data\adoreble-watchdog.log"
Do
On Error Resume Next
Set http = CreateObject("WinHttp.WinHttpRequest.5.1")
http.Open "GET", "http://127.0.0.1:9770/health", False
http.SetTimeouts 1500, 1500, 1500, 1500
http.Send
code = 0
If Not Err.Number = 0 Then code = 0 Else code = http.Status
Err.Clear
If code <> 200 Then
sh.Run """" & exe & """", 0, False
WScript.Sleep 10000
Else
WScript.Sleep 15000
End If
Loop
