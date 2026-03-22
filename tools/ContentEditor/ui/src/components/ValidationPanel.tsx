import { useMemo, useState } from 'react';
import type { ValidationIssue, ValidationReport } from '../lib/chainValidation';

type ValidationFilter = 'all' | 'errors' | 'warnings';

function renderValidationText(value: unknown) {
  if (typeof value === 'string') {
    return value;
  }

  if (value === null || value === undefined) {
    return '';
  }

  if (typeof value === 'number' || typeof value === 'boolean' || typeof value === 'bigint') {
    return String(value);
  }

  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
}

export default function ValidationPanel({
  onFocusIssue,
  report,
}: {
  onFocusIssue: (issue: ValidationIssue) => void;
  report: ValidationReport;
}) {
  const [filter, setFilter] = useState<ValidationFilter>('all');

  const filteredIssues = useMemo(() => {
    if (filter === 'errors') {
      return report.issues.filter((issue) => issue.severity === 'error');
    }
    if (filter === 'warnings') {
      return report.issues.filter((issue) => issue.severity === 'warning');
    }
    return report.issues;
  }, [filter, report.issues]);

  return (
    <>
      <div className="grid gap-3 sm:grid-cols-3">
        <div className="border border-[rgba(255,94,91,0.22)] bg-[rgba(255,94,91,0.12)] px-4 py-3">
          <p className="font-headline text-xs uppercase tracking-widest text-[#ffb6b3]">Errors</p>
          <p className="mt-2 text-2xl font-semibold text-on-surface">{report.errorCount}</p>
        </div>
        <div className="border border-[rgba(245,170,49,0.22)] bg-[rgba(245,170,49,0.12)] px-4 py-3">
          <p className="font-headline text-xs uppercase tracking-widest text-[#ffd38a]">Warnings</p>
          <p className="mt-2 text-2xl font-semibold text-on-surface">{report.warningCount}</p>
        </div>
        <div className="border border-[rgba(77,70,53,0.15)] bg-surface-high px-4 py-3">
          <p className="font-headline text-xs uppercase tracking-widest text-on-surface-variant">
            Chains in scope
          </p>
          <p className="mt-2 text-2xl font-semibold text-on-surface">{report.chainStatuses.length}</p>
        </div>
      </div>

      <div className="border border-[rgba(77,70,53,0.15)] bg-surface-low p-4">
        <div className="flex flex-wrap gap-2">
          {([
            ['all', 'All issues'],
            ['errors', 'Errors only'],
            ['warnings', 'Warnings only'],
          ] satisfies Array<[ValidationFilter, string]>).map(([value, label]) => (
            <button
              className={`px-3 py-2 text-sm transition-colors ${
                filter === value
                  ? 'border border-primary/28 bg-primary/12 text-primary'
                  : 'border border-[rgba(77,70,53,0.15)] bg-surface-high text-on-surface-variant'
              }`}
              key={value}
              onClick={() => setFilter(value)}
              type="button"
            >
              {label}
            </button>
          ))}
        </div>

        <div className="mt-4 space-y-3">
          {filteredIssues.length ? (
            filteredIssues.map((issue) => (
              <button
                className={`w-full border p-4 text-left transition-colors ${
                  issue.severity === 'error'
                    ? 'border-[rgba(255,94,91,0.22)] bg-[rgba(255,94,91,0.08)] hover:bg-[rgba(255,94,91,0.12)]'
                    : 'border-[rgba(245,170,49,0.22)] bg-[rgba(245,170,49,0.08)] hover:bg-[rgba(245,170,49,0.12)]'
                }`}
                key={issue.id}
                onClick={() => onFocusIssue(issue)}
                type="button"
              >
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <p
                      className={`text-xs font-semibold uppercase tracking-[0.18em] ${
                        issue.severity === 'error' ? 'text-[#ffb6b3]' : 'text-[#ffd38a]'
                      }`}
                    >
                      {renderValidationText(issue.scope)}
                    </p>
                    <p className="mt-2 text-sm font-semibold text-on-surface">
                      {renderValidationText(issue.title)}
                    </p>
                    <p className="mt-2 text-sm leading-6 text-on-surface-variant">
                      {renderValidationText(issue.message)}
                    </p>
                  </div>
                  <span className="border border-[rgba(77,70,53,0.15)] bg-surface-lowest px-3 py-1 text-[11px] font-medium uppercase tracking-[0.16em] text-on-surface-variant">
                    {issue.severity}
                  </span>
                </div>
                <div className="mt-3 flex flex-wrap gap-2 text-xs text-on-surface-variant">
                  {issue.chainName ? (
                    <span className="border border-[rgba(77,70,53,0.15)] bg-surface-high px-3 py-1">
                      {renderValidationText(issue.chainName)}
                    </span>
                  ) : null}
                  {issue.nodeTitle ? (
                    <span className="border border-[rgba(77,70,53,0.15)] bg-surface-high px-3 py-1">
                      {renderValidationText(issue.nodeTitle)}
                    </span>
                  ) : null}
                  {issue.sequenceId ? (
                    <span className="border border-[rgba(77,70,53,0.15)] bg-surface-high px-3 py-1">
                      {renderValidationText(issue.sequenceId)}
                    </span>
                  ) : null}
                  <span className="border border-[rgba(77,70,53,0.15)] bg-surface-high px-3 py-1">
                    Focus
                  </span>
                </div>
              </button>
            ))
          ) : (
            <div className="border border-[rgba(34,197,94,0.22)] bg-[rgba(34,197,94,0.1)] px-4 py-4">
              <p className="text-sm font-semibold text-[#c7ffd5]">Validation clean</p>
              <p className="mt-2 text-sm leading-6 text-on-surface-variant">
                The current scope has no blocking issues or warnings.
              </p>
            </div>
          )}
        </div>
      </div>

      <div className="border border-[rgba(77,70,53,0.15)] bg-surface-low p-4">
        <p className="font-headline text-xs uppercase tracking-widest text-primary">
          Chain status
        </p>
        <div className="mt-4 space-y-2">
          {report.chainStatuses.map((chain) => (
            <div
              className="flex items-center justify-between border border-[rgba(77,70,53,0.15)] bg-surface-high px-3 py-3"
              key={chain.chainId}
            >
              <div>
                <p className="text-sm font-medium text-on-surface">
                  {renderValidationText(chain.chainName)}
                </p>
              </div>
              <div className="flex gap-2 text-xs">
                <span className="border border-[rgba(255,94,91,0.22)] bg-[rgba(255,94,91,0.12)] px-3 py-1 text-[#ffb6b3]">
                  {chain.errorCount} errors
                </span>
                <span className="border border-[rgba(245,170,49,0.22)] bg-[rgba(245,170,49,0.12)] px-3 py-1 text-[#ffd38a]">
                  {chain.warningCount} warnings
                </span>
              </div>
            </div>
          ))}
        </div>
      </div>
    </>
  );
}
