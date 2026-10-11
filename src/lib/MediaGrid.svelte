<script lang="ts">
  import { Image, Play, Heart, CloudOff, AlertCircle } from 'lucide-svelte';
  import {media} from './api';
  import {duration,type Asset} from './types';
  let {assets,timeline=false,open,loadMore,hasMore=false,loading=false}:{assets:Asset[];timeline?:boolean;open:(asset:Asset)=>void;loadMore:()=>void;hasMore?:boolean;loading?:boolean}=$props();
  let width=$state(900),height=$state(600),scrollTop=$state(0);
  let columns=$derived(Math.max(2,Math.floor(width/205)));
  let rows=$derived.by(()=>{
    let rows:{top:number;height:number;label?:string;items?:Asset[]}[]=[],top=0,month='';
    let pending:Asset[]=[];
    const flush=()=>{if(pending.length){rows.push({top,height:218,items:pending});top+=218;pending=[];}};
    for(const asset of assets){
      const next=asset.effective_date.slice(0,7);
      if(timeline&&next!==month){flush();month=next;const label=new Date(next+'-01T12:00:00').toLocaleDateString(undefined,{month:'long',year:'numeric'});rows.push({top,height:54,label});top+=54;}
      pending.push(asset);if(pending.length===columns)flush();
    }flush();return {rows,total:top};
  });
  let visible=$derived(rows.rows.filter(r=>r.top+r.height>scrollTop-300&&r.top<scrollTop+height+300));
  function scroll(event:Event){const el=event.currentTarget as HTMLElement;scrollTop=el.scrollTop;if(el.scrollHeight-el.scrollTop-el.clientHeight<700&&hasMore&&!loading)loadMore();}
</script>
<div class="grid-scroll" bind:clientWidth={width} bind:clientHeight={height} onscroll={scroll}>
  <div class="virtual-grid" style:height={`${rows.total}px`}>
    {#each visible as row (row.top)}
      <div class="media-row" style:top={`${row.top}px`} style:height={`${row.height}px`} style:grid-template-columns={`repeat(${columns},minmax(0,1fr))`}>
        {#if row.label}<h2 class="month-heading">{row.label}</h2>{:else}
          {#each row.items??[] as asset (asset.id)}
            <button class="media-card" onclick={()=>open(asset)} title={asset.filename}>
              <div class="image-wrap">
                {#if asset.thumbnail_state==='ready'}<img src={media('thumb',asset.id,asset.revision)} alt={asset.filename} loading="lazy"/>
                {:else}<div class="placeholder"><Image size={30} strokeWidth={1}/><span>{asset.thumbnail_state==='budget_full'?'Storage limit reached':asset.metadata_state==='failed'?'Preview unavailable':'Preparing preview'}</span></div>{/if}
                {#if asset.kind==='video'}<span class="video-badge"><Play size={10} fill="currentColor"/>{duration(asset.duration)}</span>{/if}
                {#if asset.favorite}<span class="favorite-badge"><Heart size={14} fill="currentColor"/></span>{/if}
                {#if asset.availability!=='present'}<span class="offline-badge" title="Original unavailable"><CloudOff size={13}/></span>{/if}
                {#if asset.error}<span class="error-badge" title={asset.error}><AlertCircle size={14}/></span>{/if}
              </div>
              <div class="card-caption"><span>{asset.filename}</span><small>{asset.effective_date.slice(0,10)}</small></div>
            </button>
          {/each}
        {/if}
      </div>
    {/each}
  </div>
  {#if hasMore}<div class="load-more"><button class="secondary" onclick={loadMore} disabled={loading}>{loading?'Loading…':'Load more moments'}</button></div>{/if}
</div>
