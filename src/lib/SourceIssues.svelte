<script lang="ts">
  import {call} from './api';
  import type {Source,SourceIssue,SourceIssuePage} from './types';

  let {source}:{source:Source}=$props();
  let expanded=$state(false),loading=$state(false),error=$state('');
  let issues=$state<SourceIssue[]>([]),total=$state(0),cursor=$state<string|null>(null);
  let request=0;
  const labels:Record<string,string>={folder_unreadable:'Could not read this folder',file_unreadable:'Could not read this file',scan_interrupted:'Scan interrupted'};

  async function load(more=false){
    const ticket=++request;
    loading=true;error='';
    try{
      const page=await call<SourceIssuePage>('source_issues',{id:source.id,cursor:more?cursor:null});
      if(ticket!==request)return;
      issues=more?[...issues,...page.issues]:page.issues;
      total=page.total;cursor=page.next_cursor;
    }catch(e){
      if(ticket!==request)return;
      if(more&&String(e).includes('scan_changed')){await load();}
      else error=String(e).replace(/^Error: /,'');
    }finally{if(ticket===request)loading=false;}
  }
  function toggle(){expanded=!expanded;if(expanded)void load();}
</script>

<button class="text-button" aria-expanded={expanded} aria-controls={`scan-issues-${source.id}`} onclick={toggle}>
  {expanded?'Hide':'Show'} scan issues ({source.scan_errors})
</button>
{#if expanded}
  <section class="scan-issues" id={`scan-issues-${source.id}`} aria-label={`Scan issues for ${source.name}`} aria-busy={loading}>
    <p>Check folder permissions and reconnect the source if needed, then rescan. Previously indexed moments stay in your catalog.</p>
    <button class="text-button" disabled={loading} onclick={()=>load()}>Refresh report</button>
    {#if error}<p role="alert">{error}</p>{/if}
    <ul>
      {#each issues as issue (issue.id)}
        <li>
          <strong>{issue.path||'Source folder'}</strong>
          <span>{labels[issue.code]??issue.code}</span>
          <p>{issue.message||'Rescan to collect the path and error details for this older report.'}</p>
        </li>
      {/each}
    </ul>
    <p role="status">{loading?'Loading scan issues…':total===0?'No scan issues in this report.':`Showing ${issues.length} of ${total} scan issues.`}</p>
    {#if cursor}<button class="secondary small" disabled={loading} onclick={()=>load(true)}>Load more issues</button>{/if}
  </section>
{/if}

<style>
  .scan-issues{margin-top:8px;padding:14px;border:1px solid var(--line);border-radius:7px;overflow-wrap:anywhere}
  .scan-issues p{font-size:11px;color:#69785e;line-height:1.7;margin:0 0 8px}
  ul{list-style:none;margin:4px 0 12px;padding:0;max-height:300px;overflow:auto}
  li{padding:12px 0;border-bottom:1px solid var(--line)}
  li strong,li span{display:block;font-size:11px;margin-bottom:5px}
  li strong{white-space:pre-wrap}
  li span{color:#98633f}
  li p{white-space:pre-wrap;margin:0}
</style>
