export interface Source { id:string;name:string;path:string;volume_id:string;state:string;scan_state:string;total:number;errors:number;media_errors:number;scan_errors:number;last_scan:string|null;exclusions:string[] }
export interface SourceIssue {id:number;path:string|null;code:string;message:string}
export interface SourceIssuePage {issues:SourceIssue[];total:number;next_cursor:string|null}
export interface Asset { id:string;source_id:string;source_name:string;filename:string;folder:string;kind:string;size:number;modified:string;added:string;captured:string|null;date_source:string;effective_date:string;revision:string;width:number|null;height:number|null;duration:number|null;camera:string|null;latitude:number|null;longitude:number|null;metadata_state:string;thumbnail_state:string;error:string|null;availability:string;favorite:boolean;caption:string;tags:string[];place:string;date_override:string;exclude_memory:boolean }
export interface Annotation { favorite:boolean;caption:string;tags:string[];place:string;date_override:string;exclude_memory:boolean }
export interface Album {id:string;name:string;count:number}
export interface Collection {id:string;name:string;query:Query}
export interface Settings {thumbnail_budget:number;memories_enabled:boolean;excluded_dates:[string,string][]}
export interface Overview {sources:Source[];albums:Album[];collections:Collection[];settings:Settings;total:number;photos:number;videos:number;favorites:number;bytes:number;thumbnail_bytes:number;folders:string[];places:string[]}
export interface Query {text:string;source:string;kind:string;favorites:boolean;availability:string;date_from:string;date_to:string;place:string;folder:string;album:string;memory:boolean;errors:boolean;gps:boolean;sort:string;ascending:boolean;cursor:string|null;limit:number}
export interface Page {assets:Asset[];total:number;next_cursor:string|null}
export const defaultQuery=():Query=>({text:'',source:'',kind:'',favorites:false,availability:'',date_from:'',date_to:'',place:'',folder:'',album:'',memory:false,errors:false,gps:false,sort:'capture',ascending:false,cursor:null,limit:120});
export const emptyOverview=():Overview=>({sources:[],albums:[],collections:[],settings:{thumbnail_budget:1073741824,memories_enabled:true,excluded_dates:[]},total:0,photos:0,videos:0,favorites:0,bytes:0,thumbnail_bytes:0,folders:[],places:[]});
export const bytes=(n:number)=>n>=1e12?`${(n/1e12).toFixed(1)} TB`:n>=1e9?`${(n/1e9).toFixed(1)} GB`:n>=1e6?`${(n/1e6).toFixed(1)} MB`:`${Math.max(0,n/1e3).toFixed(0)} KB`;
export const date=(value:string)=>value?new Intl.DateTimeFormat(undefined,{month:'long',day:'numeric',year:'numeric'}).format(new Date(value.slice(0,10)+'T12:00:00')):'Unknown date';
export const duration=(n:number|null)=>n===null?'Video':`${Math.floor(n/60)}:${String(Math.floor(n%60)).padStart(2,'0')}`;
