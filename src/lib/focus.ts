export function focusTrap(node:HTMLElement) {
  const previous=document.activeElement as HTMLElement|null;
  const candidates=()=>Array.from(node.querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled),select:not(:disabled),textarea:not(:disabled),[tabindex="0"]')).filter(e=>e.getClientRects().length>0);
  queueMicrotask(()=>candidates()[0]?.focus());
  function trap(event:KeyboardEvent){
    if(event.key!=='Tab')return;
    const list=candidates();const first=list[0],last=list[list.length-1];
    if(!first){event.preventDefault();node.focus();return;}
    if(event.shiftKey&&(document.activeElement===first||document.activeElement===node)){event.preventDefault();last.focus();}
    else if(!event.shiftKey&&document.activeElement===last){event.preventDefault();first.focus();}
  }
  node.addEventListener('keydown',trap);
  return {destroy(){node.removeEventListener('keydown',trap);previous?.focus();}};
}
