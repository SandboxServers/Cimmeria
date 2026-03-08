const WS_URL = import.meta.env.VITE_WS_URL || 'ws://localhost:8443';

/**
 * Create a WebSocket connection to the admin API event stream.
 *
 * @param path - WebSocket endpoint path (e.g. "/ws/events", "/ws/logs")
 * @param onMessage - Callback invoked for each incoming message
 * @returns A cleanup function that closes the socket
 */
export function connectWs(
    path: string,
    onMessage: (data: unknown) => void,
): () => void {
    const url = `${WS_URL}${path}`;
    const ws = new WebSocket(url);

    ws.onopen = () => {
        console.log(`[ws] connected to ${url}`);
    };

    ws.onmessage = (event) => {
        try {
            const data = JSON.parse(event.data);
            onMessage(data);
        } catch {
            onMessage(event.data);
        }
    };

    ws.onerror = (err) => {
        console.error(`[ws] error on ${url}`, err);
    };

    ws.onclose = () => {
        console.log(`[ws] disconnected from ${url}`);
    };

    return () => {
        ws.close();
    };
}
