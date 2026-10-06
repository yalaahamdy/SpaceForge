@echo off
rem SpaceForge quick build wrapper
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0build.ps1" %*
