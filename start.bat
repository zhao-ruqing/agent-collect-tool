@echo off
chcp 65001 >nul
title Agent Collect Tool - 一键启动

echo ============================================
echo   Agent Collect Tool - 启动中...
echo ============================================
echo.

:: 1. 检查 MySQL
echo [1/3] 检查 MySQL...
D:\javatools\mysql\bin\mysqladmin.exe -u root -proot ping >nul 2>&1
if %errorlevel% neq 0 (
    echo   MySQL 未启动，正在启动...
    net start MySQL 2>nul
    timeout /t 3 /nobreak >nul
)
echo   MySQL OK
echo.

:: 2. 启动后端
echo [2/3] 启动后端 (localhost:3000)...
start "Backend" /MIN cmd /c "cd /d D:\Project\agent-collect-tool\backend && cargo run"
echo   等待后端就绪...
timeout /t 6 /nobreak >nul
echo   后端已启动
echo.

:: 3. 启动管理后台
echo [3/3] 启动管理后台 (localhost:5173)...
start "Admin-UI" /MIN cmd /c "cd /d D:\Project\agent-collect-tool\admin-ui && npm run dev"
timeout /t 4 /nobreak >nul
echo   管理后台已启动
echo.

echo ============================================
echo   启动完成！
echo   管理后台: http://localhost:5173
echo   后端 API:  http://localhost:3000
echo ============================================
echo.
echo  Agent 服务单独管理:
echo   安装: agent.exe install  (需管理员)
echo   启动: sc start AgentCollectTool
echo.
echo 关闭本窗口不会影响前后端运行。
pause
