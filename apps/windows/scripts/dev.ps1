# Hot-reload development for Windows WinUI 3 + C#
#
# Prerequisites:
#   - .NET 10 SDK installed
#   - Visual Studio 2022 or dotnet CLI
#
# WinUI 3 supports Hot Reload via:
#   1. .NET Hot Reload — dotnet watch run with XAML Hot Reload enabled
#   2. XAML Hot Reload — Visual Studio or standalone via dotnet CLI
#
# Usage:
#   .\dev.ps1
#
# Note: This is a stub implementation. The WinUI 3 client is not yet implemented.
#       Once scaffolded, use the following pattern for hot reload:

# .\dev.ps1 — Recommended approach

param(
    [string]$ProjectPath = "."
)

Write-Host "hwLedger Windows WinUI 3 Hot-Reload Development"
Write-Host "Note: This requires the WinUI 3 client to be scaffolded first."
Write-Host ""
Write-Host "To implement hot reload once the project is ready:"
Write-Host ""
Write-Host "1. Using dotnet watch (rebuilds + relaunches):"
Write-Host "   dotnet watch run"
Write-Host ""
Write-Host "2. Using Visual Studio XAML Hot Reload:"
Write-Host "   - Open the project in Visual Studio 2022"
Write-Host "   - Enable Debug > XAML Hot Reload"
Write-Host "   - Edit XAML or C# code; changes appear instantly"
Write-Host ""
Write-Host "3. Using .NET Hot Reload CLI:"
Write-Host "   dotnet watch --project $ProjectPath"
Write-Host ""
Write-Host "See apps/windows/ for project structure."
