import { Data, Effect, Schema } from 'effect';

const Notes = Schema.Struct({schema_version: Schema.Literal(1), patches: Schema.Array(Schema.Struct({
  id: Schema.String, title: Schema.String, description: Schema.NullOr(Schema.String),
}))});
const messages = {
  network: 'Patch notes could not be downloaded. Check your connection and try again.',
  too_large: 'The release manifest exceeds the supported size.',
  signature: 'The release manifest could not be authenticated. No patch notes were loaded.',
  signing_key_unavailable: 'This build has no release verification key. Use a configured launcher build.',
  invalid_manifest: 'The release manifest is not supported by this launcher.',
  schema: 'The launcher returned an unsupported patch-notes response.',
} as const;
type NotesCode = keyof typeof messages;
class NotesFailure extends Data.TaggedError('NotesFailure')<{readonly code:NotesCode}> {}

/** Independent read-only workflow: no settings locks and no installed-state claims. */
export function mountPatchNotes(document: Document, invoke: () => Promise<unknown>) {
  const entries = document.getElementById('entries')!;
  const status = document.getElementById('notes-status')!;
  const refresh = document.getElementById('notes-refresh')! as HTMLButtonElement;
  const abort = new AbortController();
  let disposed = false;
  let loading = false;
  let loaded = false;
  let pending: Promise<void> = Promise.resolve();
  const load = () => {
    if(disposed || loading) return;
    loading=true; refresh.disabled=true;
    status.textContent='Checking the signed release manifest…';
    const effect = Effect.tryPromise({try:invoke, catch: error => new NotesFailure({
      code:typeof error === 'string' && Object.hasOwn(messages,error) ? error as NotesCode : 'network',
    })}).pipe(
      Effect.timeout('35 seconds'),
      Effect.catchTag('TimeoutError', () => Effect.fail(new NotesFailure({code:'network'}))),
      Effect.flatMap(value => Schema.decodeUnknownEffect(Notes)(value, {onExcessProperty:'error'}).pipe(
        Effect.mapError(() => new NotesFailure({code:'schema'})),
      )),
      Effect.match({
        onFailure: error => {
          if(disposed) return;
          // Old verified notes are retained, explicitly labelled when refresh fails.
          status.textContent = `${messages[error.code]}${loaded ? ' Showing the previously verified notes from this session.' : ''}`;
        },
        onSuccess: notes => {
          if(disposed) return;
          entries.replaceChildren();
          for(const patch of notes.patches) {
            const article=document.createElement('article');
            const title=document.createElement('h3'); title.textContent=patch.title;
            const id=document.createElement('small'); id.textContent=patch.id;
            article.append(title,id);
            if(patch.description) { const description=document.createElement('p'); description.textContent=patch.description; article.append(description); }
            entries.append(article);
          }
          loaded=true;
          status.textContent=notes.patches.length ? 'Verified release manifest. These are available patches, not an installation record.' : 'The verified release manifest lists no patches.';
        },
      }),
    );
    // No automatic retries: Refresh is explicit, and each native fetch is bounded.
    pending=Effect.runPromise(effect,{signal:abort.signal}).catch(() => {}).finally(() => {
      loading=false;
      if(!disposed) refresh.disabled=false;
    });
  };
  refresh.addEventListener('click',load);
  return {
    open: () => {if(!loaded) load();},
    settled: () => pending,
    dispose: async () => {disposed=true; abort.abort(); refresh.removeEventListener('click',load); await pending;},
  };
}
