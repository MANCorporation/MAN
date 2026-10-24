@echo off
rem Runs once at first logon: copy the agent and start it at every logon with admin rights.
for %%d in (D E F G H) do if exist %%d:\agent.ps1 copy /y %%d:\agent.ps1 C:\agent.ps1
schtasks /create /tn MANAgent /sc onlogon /ru man /rl highest /it /tr "powershell -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File C:\agent.ps1" /f
powercfg /change monitor-timeout-ac 0
powercfg /change standby-timeout-ac 0
powercfg /h off
reg add "HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Power" /v HiberbootEnabled /t REG_DWORD /d 0 /f
schtasks /run /tn MANAgent
