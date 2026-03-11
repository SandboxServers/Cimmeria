const BASE_URL = import.meta.env.VITE_API_URL || '';

export async function fetchJson<T>(path: string): Promise<T> {
    const res = await fetch(`${BASE_URL}/api${path}`);
    return res.json();
}
