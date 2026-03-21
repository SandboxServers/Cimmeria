import { useState } from 'react';
import { ArrowRightLeft, Download, Database, X, AlertTriangle, CheckCircle2 } from 'lucide-react';
import { invoke } from '../lib/tauri';

interface ConvertResult {
  chains: unknown[];
  warnings: string[];
  space_name: string;
}

interface ConvertScriptDialogProps {
  scriptPath: string;
  onClose: () => void;
  onStatus: (msg: string) => void;
}

export default function ConvertScriptDialog({
  scriptPath,
  onClose,
  onStatus,
}: ConvertScriptDialogProps) {
  const [startChainId, setStartChainId] = useState('1000');
  const [scopeType, setScopeType] = useState('space');
  const [scopeId, setScopeId] = useState('');
  const [converting, setConverting] = useState(false);
  const [result, setResult] = useState<ConvertResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const fileName = scriptPath.split('/').pop() ?? scriptPath;

  const handleConvert = async () => {
    setConverting(true);
    setError(null);
    setResult(null);
    try {
      const res = await invoke<ConvertResult>('convert_script_to_chains', {
        path: scriptPath,
        options: {
          start_chain_id: startChainId ? parseInt(startChainId) : null,
          scope_type: scopeType || null,
          scope_id: scopeId ? parseInt(scopeId) : null,
        },
      });
      setResult(res);
    } catch (e) {
      setError(String(e));
    } finally {
      setConverting(false);
    }
  };

  const handleSaveToDB = async () => {
    if (!result) return;
    try {
      const ids = await invoke<number[]>('save_chains', { chains: result.chains });
      onStatus(`Saved ${ids.length} chains to database (IDs: ${ids[0]}..${ids[ids.length - 1]})`);
      onClose();
    } catch (e) {
      setError(`Save failed: ${e}`);
    }
  };

  const handleExportSQL = async () => {
    if (!result) return;
    // Save chains to DB first, then export
    try {
      const ids = await invoke<number[]>('save_chains', { chains: result.chains });
      const spaceId = `${scopeType}:${scopeId || 'global'}`;
      // Use space_<name>_chains.sql format (Windows-safe, no colons)
      const safeName = result.space_name.toLowerCase().replace(/\s+/g, '_');
      const filename = `space_${safeName}_chains.sql`;
      const path = await invoke<string>('export_to_seed_file', { spaceId, filename });
      onStatus(`Exported ${ids.length} chains to ${path}`);
      onClose();
    } catch (e) {
      setError(`Export failed: ${e}`);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60" onClick={onClose}>
      <div
        className="relative w-[520px] border border-[rgba(77,70,53,0.15)] bg-surface-low shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-[rgba(77,70,53,0.15)] px-5 py-4">
          <div className="flex items-center gap-2.5">
            <ArrowRightLeft className="h-5 w-5 text-primary" />
            <h2 className="font-headline text-sm uppercase tracking-widest text-primary">Convert Script to Chains</h2>
          </div>
          <button onClick={onClose} className="p-1 hover:bg-surface-highest">
            <X className="h-4 w-4 text-on-surface-variant" />
          </button>
        </div>

        {/* Body */}
        <div className="space-y-4 px-5 py-4">
          <div className="border border-[rgba(77,70,53,0.15)] bg-secondary-container/20 px-3 py-2 text-xs text-on-surface-variant">
            {fileName}
          </div>

          {/* Options */}
          {!result && (
            <div className="grid grid-cols-3 gap-3">
              <label className="space-y-1">
                <span className="text-xs font-dense uppercase tracking-widest text-primary">
                  Start Chain ID
                </span>
                <input
                  type="number"
                  value={startChainId}
                  onChange={(e) => setStartChainId(e.target.value)}
                  className="w-full border border-[rgba(77,70,53,0.15)] bg-surface-lowest px-2.5 py-1.5 text-sm text-on-surface focus:border-primary focus:outline-none"
                />
              </label>
              <label className="space-y-1">
                <span className="text-xs font-dense uppercase tracking-widest text-primary">
                  Scope Type
                </span>
                <select
                  value={scopeType}
                  onChange={(e) => setScopeType(e.target.value)}
                  className="w-full border border-[rgba(77,70,53,0.15)] bg-surface-lowest px-2.5 py-1.5 text-sm text-on-surface focus:border-primary focus:outline-none"
                >
                  <option value="space">space</option>
                  <option value="mission">mission</option>
                  <option value="global">global</option>
                </select>
              </label>
              <label className="space-y-1">
                <span className="text-xs font-dense uppercase tracking-widest text-primary">
                  Scope ID
                </span>
                <input
                  type="number"
                  value={scopeId}
                  onChange={(e) => setScopeId(e.target.value)}
                  placeholder="e.g. 8"
                  className="w-full border border-[rgba(77,70,53,0.15)] bg-surface-lowest px-2.5 py-1.5 text-sm text-on-surface placeholder:text-on-surface-variant/50 focus:border-primary focus:outline-none"
                />
              </label>
            </div>
          )}

          {/* Error */}
          {error && (
            <div className="border border-error/30 bg-error/10 px-3 py-2 text-xs text-error">
              {error}
            </div>
          )}

          {/* Results */}
          {result && (
            <div className="space-y-3">
              <div className="flex items-center gap-2 text-sm">
                <CheckCircle2 className="h-4 w-4 text-green-500" />
                <span>
                  Converted <strong>{result.chains.length}</strong> chains from{' '}
                  <strong>{result.space_name}</strong>
                </span>
              </div>

              {result.warnings.length > 0 && (
                <div className="space-y-1">
                  <div className="flex items-center gap-1.5 text-xs font-medium text-yellow-500">
                    <AlertTriangle className="h-3.5 w-3.5" />
                    {result.warnings.length} warning{result.warnings.length !== 1 ? 's' : ''}
                  </div>
                  <div className="subtle-scrollbar max-h-32 overflow-y-auto border border-[rgba(77,70,53,0.15)] bg-secondary-container/20 px-3 py-2">
                    {result.warnings.map((w, i) => (
                      <div key={i} className="text-[11px] text-on-surface-variant">
                        {w}
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-2 border-t border-[rgba(77,70,53,0.15)] px-5 py-3">
          {!result ? (
            <>
              <button
                onClick={onClose}
                className="bg-surface-high px-3 py-1.5 text-sm text-on-surface-variant hover:bg-surface-highest"
              >
                Cancel
              </button>
              <button
                onClick={handleConvert}
                disabled={converting}
                className="flex items-center gap-1.5 bg-primary-container px-4 py-1.5 text-sm font-medium text-on-primary bevel-stone transition-colors hover:bg-primary-container/90 disabled:opacity-50"
              >
                <ArrowRightLeft className="h-3.5 w-3.5" />
                {converting ? 'Converting...' : 'Convert'}
              </button>
            </>
          ) : (
            <>
              <button
                onClick={() => { setResult(null); setError(null); }}
                className="bg-surface-high px-3 py-1.5 text-sm text-on-surface-variant hover:bg-surface-highest"
              >
                Back
              </button>
              <button
                onClick={handleSaveToDB}
                className="flex items-center gap-1.5 bg-primary-container px-4 py-1.5 text-sm font-medium text-on-primary bevel-stone transition-colors hover:bg-primary-container/90"
              >
                <Database className="h-3.5 w-3.5" />
                Save to DB
              </button>
              <button
                onClick={handleExportSQL}
                className="flex items-center gap-1.5 border border-[rgba(77,70,53,0.15)] bg-surface-high px-4 py-1.5 text-sm font-medium text-on-surface transition-colors hover:bg-surface-highest"
              >
                <Download className="h-3.5 w-3.5" />
                Export SQL
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
