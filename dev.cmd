@echo off
rem FRAMEWORK FILE. Runs dev.ps1 even when PowerShell script execution is disabled.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0dev.ps1" %*
