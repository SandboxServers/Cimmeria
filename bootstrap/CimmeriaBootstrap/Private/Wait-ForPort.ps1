function Wait-ForPort {
    <#
    .SYNOPSIS
        Polls a TCP port until it accepts connections or times out.
        Returns $true if the port is ready, $false on timeout.
    #>
    param(
        [Parameter(Mandatory)][int]$Port,
        [int]$TimeoutSeconds = 15,
        [int]$IntervalMs = 500
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        try {
            $tcp = [System.Net.Sockets.TcpClient]::new()
            $tcp.Connect("127.0.0.1", $Port)
            $tcp.Close()
            return $true
        } catch {
            Start-Sleep -Milliseconds $IntervalMs
        }
    }
    return $false
}
