<script lang="ts">
  import {onMount} from 'svelte';
  import {focusTrap} from './lib/focus';
  import {Grid2X2,Clock3,Heart,Album as AlbumIcon,HardDrive,Plus,Search,SlidersHorizontal,Settings as SettingsIcon,ShieldCheck,ArrowUpDown,FolderOpen,X,ChevronLeft,ChevronRight,Play,Pause,RefreshCw,MapPin,Image,Video,Download,ExternalLink,Check,AlertCircle,Sun,ArrowRight,Info,Folder,Trash2,Link,CheckCircle2} from 'lucide-svelte';
  import MediaGrid from './lib/MediaGrid.svelte';
  import {call,desktop,media} from './lib/api';
  import {defaultQuery,emptyOverview,bytes,date,duration,type Asset,type Annotation,type Overview,type Page,type Source} from './lib/types';
  let overview=$state<Overview>(emptyOverview()),q=$state(defaultQuery()),assets=$state<Asset[]>([]),total=$state(0),cursor=$state<string|null>(null);
  let view=$state('library'),loading=$state(false),busy=$state(false),error=$state(''),notice=$state(''),filters=$state(false),searchInput=$state('');
  let selected=$state<Asset|null>(null),annotation=$state<Annotation>({favorite:false,caption:'',tags:[],place:'',date_override:'',exclude_memory:false}),tagsInput=$state('');
  let modal=$state<''|'sources'|'settings'|'album'>(''),albumName=$state(''),sourceEdit=$state<Source|null>(null),sourceName=$state(''),sourceExclusions=$state('');
  let budget=$state(1024),memoriesEnabled=$state(true),excludedFrom=$state(''),excludedTo=$state(''),excludedDates=$state<[string,string][]>([]);
  let previewFailed=$state(false),playbackFailed=$state(false),confirmAction=$state<{title:string;body:string;run:()=>Promise<void>}|null>(null);
  let generation=0,searchTimer:ReturnType<typeof setTimeout>,noticeTimer:ReturnType<typeof setTimeout>;
  let heading=$derived(view==='memories'?'A little trip back':view==='favorites'?'Your favorites':view==='timeline'?'Through the years':view==='album'?(overview.albums.find(a=>a.id===q.album)?.name??'Album'):view==='source'?(overview.sources.find(s=>s.id===q.source)?.name??'Source'):'Your library');
  let scanning=$derived(overview.sources.some(s=>['scanning','queued'].includes(s.scan_state)));
  let hasFilters=$derived(Boolean(q.kind||q.availability||q.date_from||q.date_to||q.place||q.folder||q.errors||q.gps));
  function message(text:string){notice=text;clearTimeout(noticeTimer);noticeTimer=setTimeout(()=>notice='',5000);}
  async function guard(fn:()=>Promise<void>){try{error='';await fn();}catch(e){error=String(e).replace(/^Error: /,'');}}
  async function refresh(){if(!desktop)return;overview=await call<Overview>('overview');}
  async function refreshViewer(){
    const id=selected?.id;if(!id)return;
    const latest=await call<Asset>('get_asset',{id});
    if(selected?.id!==id)return;
    if(selected.availability!==latest.availability||selected.revision!==latest.revision){previewFailed=false;playbackFailed=false;}
    // Keep the separate annotation form intact while source availability changes.
    selected=latest;
  }
  async function load(more=false){
    if(!desktop)return;const ticket=++generation;loading=true;
    try{const page=await call<Page>('query',{query:{...q,cursor:more?cursor:null}});if(ticket!==generation)return;assets=more?[...assets,...page.assets]:page.assets;total=page.total;cursor=page.next_cursor;}
    catch(e){if(String(e).includes('library_changed')&&more){await load();}else if(ticket===generation)error=String(e);}
    finally{if(ticket===generation)loading=false;}
  }
  function navigate(next:string,id=''){
    view=next;searchInput='';q={...defaultQuery(),favorites:next==='favorites',memory:next==='memories',source:next==='source'?id:'',album:next==='album'?id:''};
    selected=null;void load();
  }
  function filterChanged(){void load();}
  function search(){clearTimeout(searchTimer);searchTimer=setTimeout(()=>{q.text=searchInput;void load();},220);}
  async function addSource(){busy=true;await guard(async()=>{const id=await call<string|null>('add_source');if(id){await refresh();navigate('source',id);message('Folder connected. Your library is taking shape.');}});busy=false;}
  function open(asset:Asset){selected=asset;annotation={favorite:asset.favorite,caption:asset.caption,tags:[...asset.tags],place:asset.place,date_override:asset.date_override,exclude_memory:asset.exclude_memory};tagsInput=asset.tags.join(', ');previewFailed=false;playbackFailed=false;}
  async function favorite(asset:Asset){await guard(async()=>{await call('annotate',{id:asset.id,annotation:{favorite:!asset.favorite,caption:asset.caption,tags:asset.tags,place:asset.place,date_override:asset.date_override,exclude_memory:asset.exclude_memory}});if(selected?.id===asset.id){selected={...selected,favorite:!asset.favorite};annotation.favorite=!asset.favorite;}await refresh();await load();});}
  async function saveAnnotation(){if(!selected)return;const id=selected.id;await guard(async()=>{annotation.tags=tagsInput.split(',').map(s=>s.trim()).filter(Boolean);await call('annotate',{id,annotation});selected=await call<Asset>('get_asset',{id});await refresh();await load();message('Details saved in Bimb.');});}
  function move(delta:number){if(!selected)return;const index=assets.findIndex(a=>a.id===selected?.id);const next=assets[index+delta];if(next)open(next);}
  function editSource(source:Source){sourceEdit=source;sourceName=source.name;sourceExclusions=source.exclusions.join('\n');}
  async function scan(id:string,action:string){await guard(async()=>{await call('scan_control',{id,action});await refresh();});}
  async function sourceOperation(command:string,id:string){await guard(async()=>{await call(command,{id});await refresh();await load();});}
  function settings(){budget=Math.round(overview.settings.thumbnail_budget/1048576);memoriesEnabled=overview.settings.memories_enabled;excludedDates=[...overview.settings.excluded_dates];modal='settings';}
  async function saveSettings(){await guard(async()=>{await call('settings',{settings:{thumbnail_budget:Math.round(budget*1048576),memories_enabled:memoriesEnabled,excluded_dates:excludedDates}});await refresh();await load();message('Preferences saved.');});}
  async function addToAlbum(id:string){if(!selected||!id)return;await guard(async()=>{await call('album',{action:'add',id,name:'',assetIds:[selected!.id]});await refresh();message('Added to album.');});}
  function confirm(title:string,body:string,run:()=>Promise<void>){confirmAction={title,body,run};}
  function key(event:KeyboardEvent){
    if(event.key==='Escape'){if(confirmAction)confirmAction=null;else if(modal)modal='';else selected=null;}
    if(event.target instanceof HTMLInputElement||event.target instanceof HTMLTextAreaElement||event.target instanceof HTMLSelectElement)return;
    if(selected&&event.key==='ArrowRight')move(1);if(selected&&event.key==='ArrowLeft')move(-1);
  }
  onMount(()=>{
    void guard(async()=>{await refresh();await load();});
    const progress=setInterval(()=>{if(!desktop)return;void guard(async()=>{const wasScanning=scanning;const before=JSON.stringify(overview.sources);await refresh();if(wasScanning||scanning||before!==JSON.stringify(overview.sources)){if(selected)await refreshViewer();else await load();}});},2000);
    return()=>{clearInterval(progress);clearTimeout(searchTimer);clearTimeout(noticeTimer);};
  });
</script>

<svelte:window onkeydown={key}/>
<div class="app-shell" inert={!!selected||!!modal||!!confirmAction}>
  <aside class="sidebar">
    <button class="brand" onclick={()=>navigate('library')} aria-label="Bimb home"><span class="brand-mark"><span></span><span></span><span></span><span></span></span>bimb<span class="brand-period">.</span></button>
    <p class="sidebar-caption">A HOME FOR YOUR MOMENTS</p>
    <nav aria-label="Library navigation">
      <button class:active={view==='library'} onclick={()=>navigate('library')}><Grid2X2 size={18}/>All moments<span>{overview.total.toLocaleString()}</span></button>
      <button class:active={view==='timeline'} onclick={()=>navigate('timeline')}><Clock3 size={18}/>Timeline</button>
      <button class:active={view==='favorites'} onclick={()=>navigate('favorites')}><Heart size={18}/>Favorites<span>{overview.favorites||''}</span></button>
      <button class:active={view==='memories'} onclick={()=>navigate('memories')}><Sun size={18}/>Memories</button>
    </nav>
    <div class="section-label">YOUR ALBUMS<button class="icon-button" title="Create album" onclick={()=>{albumName='';modal='album';}}><Plus size={15}/></button></div>
    <nav aria-label="Albums">{#each overview.albums as album}<button class:active={q.album===album.id} onclick={()=>navigate('album',album.id)}><AlbumIcon size={17}/><span class="nav-name">{album.name}</span><span>{album.count}</span></button>{/each}</nav>
    {#if !overview.albums.length}<p class="sidebar-hint">Make a little collection<br/>of something you love.</p>{/if}
    <div class="section-label">CONNECTED FOLDERS<button class="icon-button" title="Manage sources" onclick={()=>modal='sources'}><SlidersHorizontal size={14}/></button></div>
    <nav aria-label="Sources">{#each overview.sources as source}<button class:active={q.source===source.id} onclick={()=>navigate('source',source.id)}><HardDrive size={17}/><span class="nav-name">{source.name}</span><i class:online={source.state==='online'} class="status-dot" title={source.state}></i></button>{/each}</nav>
    <button class="add-folder-link" onclick={addSource} disabled={busy}><Plus size={16}/>Add a folder or drive</button>
    <div class="sidebar-bottom"><div class="privacy-card"><ShieldCheck size={20}/><div><strong>Yours. Always.</strong><p>On your drives. On your terms.</p></div></div><button class="settings-button" onclick={settings}><SettingsIcon size={17}/>Preferences<span class="version">v0.1</span></button></div>
  </aside>

  <main>
    <header class="topbar"><div class="breadcrumbs">Your space<span>/</span>{view==='source'?'Folders':view==='album'?'Albums':'Library'}</div><div class="local-badge"><span></span>Private & offline</div></header>
    {#if !desktop}<div class="preview-banner">Interface preview · Launch with <code>npm run desktop</code> to connect your library.</div>{/if}
    <div class="page-header"><div><p class="eyebrow">{view==='memories'?'SOME THINGS ARE WORTH ANOTHER LOOK':'COLLECTED BY YOU. KEPT FOR YOU.'}</p><h1>{heading}</h1><p class="subtitle">{view==='memories'?'Moments from this day, in years gone by.':overview.total?`${overview.photos.toLocaleString()} photos · ${overview.videos.toLocaleString()} videos · ${bytes(overview.bytes)} of memories`:'A lifetime of moments. A place to find them.'}</p></div><button class="primary" onclick={addSource} disabled={busy}><Plus size={17}/>{busy?'Choosing folder…':'Add folder'}</button></div>
    {#if error}<div class="error-notice" role="alert"><AlertCircle size={18}/><span>{error}</span><button class="icon-button" aria-label="Dismiss error" onclick={()=>error=''}><X size={16}/></button></div>{/if}
    {#if notice}<div class="success-notice" role="status"><CheckCircle2 size={17}/>{notice}</div>{/if}
    {#if overview.total||overview.sources.length}
      <div class="toolbar"><label class="search-box"><Search size={18}/><input aria-label="Search your library" placeholder="Search filenames, tags, places…" bind:value={searchInput} oninput={search}/><kbd>⌕</kbd></label><button class:chosen={hasFilters} class="secondary" onclick={()=>filters=!filters}><SlidersHorizontal size={16}/>Filters{#if hasFilters}<i class="filter-dot"></i>{/if}</button><div class="sort-control"><ArrowUpDown size={15}/><select aria-label="Sort media" bind:value={q.sort} onchange={filterChanged}><option value="capture">Date captured</option><option value="added">Date added</option><option value="name">Filename</option><option value="size">File size</option><option value="duration">Duration</option></select><button class="icon-button" title={q.ascending?'Ascending':'Descending'} onclick={()=>{q.ascending=!q.ascending;filterChanged();}}>{q.ascending?'↑':'↓'}</button></div></div>
      {#if filters}<div class="filter-panel">
        <label>Media<select bind:value={q.kind} onchange={filterChanged}><option value="">Photos & videos</option><option value="photo">Photos</option><option value="video">Videos</option></select></label>
        <label>Availability<select bind:value={q.availability} onchange={filterChanged}><option value="">All originals</option><option value="present">Available</option><option value="offline">Drive offline</option><option value="missing">File missing</option></select></label>
        <label>From<input type="date" bind:value={q.date_from} onchange={filterChanged}/></label><label>Until<input type="date" bind:value={q.date_to} onchange={filterChanged}/></label>
        <label>Place<select bind:value={q.place} onchange={filterChanged}><option value="">All places</option>{#each overview.places as place}<option>{place}</option>{/each}</select></label>
        <label>Folder<select bind:value={q.folder} onchange={filterChanged}><option value="">All folders</option>{#each overview.folders as folder}<option>{folder}</option>{/each}</select></label>
        <label class="checkbox"><input type="checkbox" bind:checked={q.gps} onchange={filterChanged}/>Has GPS</label><label class="checkbox"><input type="checkbox" bind:checked={q.errors} onchange={filterChanged}/>Needs attention</label>
        <button class="text-button" onclick={()=>{q={...defaultQuery(),text:q.text,source:q.source,album:q.album,favorites:q.favorites,memory:q.memory};filterChanged();}}>Reset filters</button>
      </div>{/if}
      <div class="results-meta"><span>{total.toLocaleString()} {total===1?'moment':'moments'}{#if loading}<span class="loading-dot"></span>{/if}</span><div>{#if view==='album'}<button class="text-button" onclick={()=>confirm('Delete this album?','The photos and videos stay in your library.',async()=>{await call('album',{action:'delete',id:q.album,name:'',assetIds:[]});await refresh();navigate('library');})}>Delete album</button>{/if}{#if scanning}<span class="scan-badge"><span class="loading-dot"></span>Gathering your moments</span>{:else}<span class="quiet">Originals stay in their folders</span>{/if}</div></div>
    {/if}
    {#if !overview.sources.length}
      <section class="welcome"><div class="welcome-art" aria-hidden="true"><div class="art-card back"><div class="art-sun"></div><div class="art-hill"></div></div><div class="art-card front"><svg viewBox="0 0 240 185" fill="none"><rect width="240" height="185" fill="#d6ddd0"/><circle cx="176" cy="48" r="21" fill="#f4ddb1"/><path d="M0 122L66 48L147 144L195 96L240 136V185H0Z" fill="#a1b4a1"/><path d="M0 161L93 95L174 162L240 126V185H0Z" fill="#708f78"/><path d="M0 185L92 143L165 185" fill="#486b58"/></svg><div class="art-caption">somewhere worth remembering</div></div><div class="art-leaf">✳</div></div><p class="eyebrow">START WITH A FOLDER</p><h2>Your memories already have a home.<br/>Now give them a library.</h2><p>Connect a folder or external drive. Bimb brings your photos<br/>and videos together, while keeping every original right where it is.</p><button class="primary large" onclick={addSource} disabled={busy}><FolderOpen size={19}/>Choose your first folder<ArrowRight size={17}/></button><div class="welcome-promises"><span><ShieldCheck size={15}/>No account. No cloud.</span><span><HardDrive size={15}/>Your originals stay untouched</span></div></section>
    {:else if !assets.length}
      <section class="empty-results">{#if view==='memories'}<Sun size={38} strokeWidth={1}/><h2>{overview.settings.memories_enabled?'No memories for today.':'Memories are taking a rest.'}</h2><p>{overview.settings.memories_enabled?'Only known capture dates are used. Correct a date in the details panel, or come back another day.':'You can turn memories back on in Preferences.'}</p>{:else if scanning}<RefreshCw size={32} class="spin"/><h2>Your library is taking shape.</h2><p>Browse as soon as the first files are indexed. You can pause at any time.</p>{:else}<Search size={36} strokeWidth={1}/><h2>No moments here yet.</h2><p>Try another search or filter, or connect a folder with photos and videos.</p>{/if}</section>
    {:else}<MediaGrid {assets} timeline={view==='timeline'&&q.sort==='capture'} {open} loadMore={()=>void load(true)} hasMore={!!cursor} {loading}/>{/if}
    <footer class="statusbar"><span><ShieldCheck size={13}/>Everything stays on this computer and your drives</span><button onclick={()=>modal='sources'}>{overview.sources.length} {overview.sources.length===1?'source':'sources'}<span class="status-dot online"></span></button></footer>
  </main>
</div>

{#if selected}
  <div class="viewer" use:focusTrap inert={!!modal||!!confirmAction} role="dialog" aria-modal="true" aria-label="Media viewer" tabindex="-1">
    <div class="viewer-top"><button class="icon-button light" onclick={()=>selected=null} aria-label="Close viewer"><X size={21}/></button><span>{selected.filename}</span><button class="icon-button light" title="Favorite" onclick={()=>favorite(selected!)}><Heart size={20} fill={selected.favorite?'currentColor':'none'}/></button><button class="icon-button light" title="Reveal in Finder" onclick={()=>guard(async()=>{await call('open_original',{id:selected!.id,reveal:true});})}><FolderOpen size={20}/></button></div>
    <div class="viewer-body"><div class="viewer-stage"><button class="viewer-arrow prev" aria-label="Previous item" onclick={()=>move(-1)}><ChevronLeft/></button>
      {#key selected.id}
        {#if selected.availability==='present'&&selected.kind==='video'&&!playbackFailed}<video controls preload="metadata" src={media('video',selected.id)} onerror={()=>playbackFailed=true}><track kind="captions"/></video>
        {:else if selected.availability==='present'&&selected.kind==='photo'&&!previewFailed}<img class="full-preview" src={media('preview',selected.id,selected.revision)} alt={selected.filename} onerror={()=>previewFailed=true}/>
        {:else if selected.thumbnail_state==='ready'}<img class="cached-preview" src={media('thumb',selected.id,selected.revision)} alt={selected.filename}/>{:else}<div class="viewer-no-preview"><Image size={50}/><p>Preview unavailable</p></div>{/if}
      {/key}
      {#if selected.availability!=='present'}<div class="viewer-message"><HardDrive size={17}/>Cached thumbnail · Connect “{selected.source_name}” to open the original</div>{:else if previewFailed||playbackFailed}<div class="viewer-message">This format cannot be shown here.<button onclick={()=>guard(async()=>{await call('open_original',{id:selected!.id,reveal:false});})}>Open in another app<ExternalLink size={14}/></button></div>{/if}
      <button class="viewer-arrow next" aria-label="Next item" onclick={()=>move(1)}><ChevronRight/></button>
    </div><aside class="details-panel"><p class="eyebrow">THE LITTLE DETAILS</p><h2>{date(selected.effective_date)}</h2><p class="muted">{selected.date_source==='file_modified'?'Using file modification date':selected.date_source==='user'?'Your corrected date':'Recorded capture date'}</p>
      <div class="file-facts"><span>{selected.kind==='video'?'VIDEO':'PHOTO'}</span><span>{bytes(selected.size)}</span>{#if selected.width}<span>{selected.width} × {selected.height}</span>{/if}{#if selected.kind==='video'}<span>{duration(selected.duration)}</span>{/if}</div>
      {#if selected.camera}<p class="detail-line"><Image size={16}/>{selected.camera}</p>{/if}
      <p class="detail-line"><HardDrive size={16}/>{selected.source_name}</p>{#if selected.folder}<p class="detail-line wrap"><Folder size={16}/>{selected.folder}</p>{/if}
      {#if selected.latitude!==null}<p class="detail-line"><MapPin size={16}/>{selected.latitude.toFixed(5)}, {selected.longitude?.toFixed(5)}</p>{/if}
      {#if selected.error}<p class="detail-error"><AlertCircle size={15}/>{selected.error.replaceAll('_',' ')}</p>{/if}
      <hr/><label>Caption<textarea bind:value={annotation.caption} placeholder="A little story behind this moment…" rows="3" maxlength="8192"></textarea></label>
      <label>Tags<input bind:value={tagsInput} placeholder="Family, holiday, summer"/></label><label>Place<input bind:value={annotation.place} placeholder="Give this place a name" maxlength="256"/></label><label>Correct capture date<input type="date" bind:value={annotation.date_override}/></label>
      <label class="checkbox"><input type="checkbox" bind:checked={annotation.exclude_memory}/>Keep out of memories</label><button class="primary" onclick={saveAnnotation}><Check size={16}/>Save details</button>
      <label class="album-select">Add to album<select aria-label="Add to album" onchange={(e)=>{void addToAlbum(e.currentTarget.value);e.currentTarget.value='';}}><option value="">Choose an album…</option>{#each overview.albums as album}<option value={album.id}>{album.name}</option>{/each}</select></label>
      {#if view==='album'}<button class="text-button" onclick={()=>guard(async()=>{await call('album',{action:'remove',id:q.album,name:'',assetIds:[selected!.id]});selected=null;await refresh();await load();})}>Remove from this album</button>{/if}
      <p class="detail-footnote">Your changes are saved in Bimb. The original file is never modified.</p>
    </aside></div>
  </div>
{/if}

{#if modal}
  <div class="modal-backdrop"><div class:wide={modal==='sources'} class="modal" use:focusTrap inert={!!confirmAction} role="dialog" aria-modal="true" aria-label={modal} tabindex="-1"><button class="modal-close icon-button" aria-label="Close dialog" onclick={()=>{modal='';sourceEdit=null;}}><X size={20}/></button>
    {#if modal==='album'}<p class="eyebrow">A LITTLE COLLECTION</p><h2>Create an album</h2><p class="muted">Gather moments together without moving a single file.</p><label>Album name<input bind:value={albumName} placeholder="e.g. Slow summer days" maxlength="128"/></label><button class="primary" disabled={!albumName.trim()} onclick={()=>guard(async()=>{const id=await call<string>('album',{action:'create',id:'',name:albumName,assetIds:[]});modal='';await refresh();navigate('album',id);})}><Plus size={16}/>Create album</button>
    {:else if modal==='sources'}<p class="eyebrow">RIGHT WHERE YOU LEFT THEM</p><h2>Your folders & drives</h2><p class="muted">Disconnect a drive whenever you need to. Its catalog and thumbnails stay here.</p>
      <div class="source-list">{#each overview.sources as source}<article class="source-card"><div class="source-card-top"><HardDrive size={23}/><div><h3>{source.name}</h3><p class="path">{source.path}</p></div><span class:connected={source.state==='online'} class="source-state">{source.state}</span></div><div class="source-stats">{source.total.toLocaleString()} moments · {source.scan_state}{#if source.errors} · {source.errors} need attention{/if}</div>
        <div class="source-actions"><button class="secondary small" onclick={()=>scan(source.id,'resume')}><RefreshCw size={13}/>{['paused','cancelled','interrupted'].includes(source.scan_state)?'Resume':'Rescan'}</button>{#if ['scanning','queued','paused'].includes(source.scan_state)}<button class="secondary small" onclick={()=>scan(source.id,source.scan_state==='paused'?'resume':'pause')}><Pause size={13}/>{source.scan_state==='paused'?'Resume':'Pause'}</button><button class="text-button" onclick={()=>scan(source.id,'cancel')}>Cancel</button>{/if}<button class="text-button" onclick={()=>sourceOperation('relink_source',source.id)}><Link size={13}/>Relink</button><button class="text-button" onclick={()=>editSource(source)}>Edit</button><button class="text-button danger" onclick={()=>confirm(`Remove ${source.name}?`,'This removes its catalog entries, album memberships, annotations and thumbnails. An automatic catalog backup is saved first. Original media stays untouched.',async()=>{await call('remove_source',{id:source.id});await refresh();navigate('library');})}>Remove</button></div>
        {#if source.errors}<button class="text-button" onclick={()=>{modal='';navigate('source',source.id);q.errors=true;void load();}}>Show media needing attention<ArrowRight size={13}/></button>{/if}
        {#if sourceEdit?.id===source.id}<div class="source-edit"><label>Name<input bind:value={sourceName}/></label><label>Excluded folders (relative paths, one per line)<textarea bind:value={sourceExclusions} rows="3" placeholder="Private\nWork/Exports"></textarea></label><button class="secondary" onclick={()=>guard(async()=>{await call('update_source',{id:source.id,name:sourceName,exclusions:sourceExclusions.split('\n').map(s=>s.trim()).filter(Boolean)});sourceEdit=null;await refresh();})}>Save source settings</button></div>{/if}
      </article>{/each}</div><button class="primary" onclick={addSource} disabled={busy}><Plus size={16}/>Add a folder or drive</button>
    {:else}<p class="eyebrow">MAKE YOURSELF AT HOME</p><h2>Preferences</h2><section class="settings-section"><h3>Offline thumbnails</h3><p>Using {bytes(overview.thumbnail_bytes)}. Existing thumbnails are kept when the budget is full.</p><label>Storage budget (MiB)<input type="number" min="0" max="102400" bind:value={budget}/></label>{#each overview.sources as source}<div class="setting-line"><span>{source.name}</span><button class="text-button" onclick={()=>confirm('Clear cached thumbnails?','Metadata and organization remain. Reconnect the source and rescan to recreate its thumbnails.',async()=>{await call('clear_thumbnails',{id:source.id});await refresh();await load();})}>Clear thumbnails</button></div>{/each}</section>
      <section class="settings-section"><h3>Memories</h3><label class="checkbox"><input type="checkbox" bind:checked={memoriesEnabled}/>Show memories from previous years</label><p>Keep a date range out of memories.</p><div class="date-range"><input aria-label="Exclude from" type="date" bind:value={excludedFrom}/><span>to</span><input aria-label="Exclude until" type="date" bind:value={excludedTo}/><button class="icon-button" title="Exclude dates" disabled={!excludedFrom||!excludedTo} onclick={()=>{excludedDates=[...excludedDates,[excludedFrom,excludedTo]];excludedFrom='';excludedTo='';}}><Plus size={17}/></button></div>{#each excludedDates as range,index}<div class="setting-line"><span>{range[0]} — {range[1]}</span><button class="icon-button" title="Remove exclusion" onclick={()=>excludedDates=excludedDates.filter((_,i)=>i!==index)}><X size={15}/></button></div>{/each}</section><button class="primary" onclick={saveSettings}><Check size={16}/>Save preferences</button>
      <section class="settings-section"><h3>Your catalog, safely kept</h3><p>Catalog backups preserve organization and metadata. They do not contain your original photos, videos, or thumbnails.</p><div class="button-row"><button class="secondary" onclick={()=>guard(async()=>{if(await call<boolean>('backup'))message('Catalog backup saved.');})}><Download size={15}/>Back up catalog</button><button class="secondary" onclick={()=>confirm('Restore a catalog?','This replaces the current catalog. A safety backup is saved first. Stop any active scans before continuing.',async()=>{if(await call<boolean>('restore')){modal='';await refresh();navigate('library');message('Catalog restored. Reconnect sources and rescan for thumbnails.');}})}>Restore</button></div><button class="text-button" onclick={()=>guard(async()=>{if(await call<boolean>('export_annotations'))message('Organization exported as JSON.');})}>Export organization as JSON<ExternalLink size={13}/></button></section><div class="privacy-note"><ShieldCheck size={19}/><p>No account, uploads, or telemetry. Your computer’s account and disk protection secure this local catalog.</p></div>
    {/if}
    {#if error}<div class="error-notice" role="alert"><AlertCircle size={16}/>{error}</div>{/if}
  </div></div>
{/if}
{#if confirmAction}<div class="modal-backdrop confirmation"><div class="modal" use:focusTrap role="alertdialog" aria-modal="true" aria-label={confirmAction.title} tabindex="-1"><h2>{confirmAction.title}</h2><p>{confirmAction.body}</p><div class="button-row"><button class="secondary" onclick={()=>confirmAction=null}>Keep as is</button><button class="primary" onclick={()=>{const action=confirmAction!;confirmAction=null;void guard(action.run);}}>Continue</button></div></div></div>{/if}
