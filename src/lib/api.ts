import {invoke,isTauri,convertFileSrc} from '@tauri-apps/api/core';
export const desktop=isTauri();
export async function call<T>(command:string,args:Record<string,unknown>={}):Promise<T> {
  if(!desktop) throw new Error('Open the Bimb desktop app to connect your folders.');
  return invoke<T>(command,args);
}
export const media=(kind:string,id:string,revision='')=>`${convertFileSrc(kind,'bimb')}/${id}?v=${encodeURIComponent(revision)}`;
