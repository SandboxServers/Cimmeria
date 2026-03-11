function getWsBaseUrl(path: string): string {
    if (import.meta.env.VITE_WS_URL) {
        return import.meta.env.VITE_WS_URL;
    }
    // Tauri desktop app: connect directly to the appropriate server
    if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
        const port = path.startsWith('/supervisor/') ? 8444 : 8443;
        return `ws://127.0.0.1:${port}`;
    }
    // Browser: derive from current page URL so Vite proxy works in dev
    const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    return `${proto}//${window.location.host}`;
}

/**
 * Create a reconnecting WebSocket connection.
 *
 * @param path - WebSocket endpoint path (e.g. "/ws/logs")
 * @param onMessage - Callback invoked for each incoming parsed JSON message
 * @param onStatus - Optional callback for connection status changes
 * @returns A cleanup function that closes the socket and stops reconnecting
 */
export function connectWs(
    path: string,
    onMessage: (data: unknown) => void,
    onStatus?: (connected: boolean) => void,
): () => void {
    const baseUrl = getWsBaseUrl(path);
    const url = `${baseUrl}${path}`;
    let ws: WebSocket | null = null;
    let stopped = false;
    let reconnectTimer: ReturnType<typeof setTimeout> | undefined;

    function connect() {
        if (stopped) return;

        ws = new WebSocket(url);

        ws.onopen = () => {
            onStatus?.(true);
        };

        ws.onmessage = (event) => {
            try {
                const data = JSON.parse(event.data);
                onMessage(data);
            } catch {
                onMessage(event.data);
            }
        };

        ws.onerror = () => {
            // Error is followed by close — reconnect handled there
        };

        ws.onclose = () => {
            onStatus?.(false);
            if (!stopped) {
                reconnectTimer = setTimeout(connect, 3000);
            }
        };
    }

    connect();

    return () => {
        stopped = true;
        clearTimeout(reconnectTimer);
        ws?.close();
    };
}
