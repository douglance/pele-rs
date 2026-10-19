/* Actual owned C callback: live Gantt arena, captured original and recovery. */
#include "pele.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <inttypes.h>
static char *copy_result(PeleResult *r) { assert(r); size_t n=pele_result_len(r); char *s=malloc(n+1); assert(s); memcpy(s,pele_result_data(r),n);s[n]=0;return s; }
static void good(PeleResult *r) { if(!r||pele_result_status(r)){char*s=copy_result(r);fprintf(stderr,"unexpected %s\n",s);abort();}pele_result_free(r); }
static void failure(PeleResult*r,const char*match){assert(r&&pele_result_status(r));char*s=copy_result(r);assert(strstr(s,match));free(s);pele_result_free(r);}
static char *object(const char *s){ assert(*s=='{'); const char*end=s;int depth=0,quote=0,escape=0;do{char c=*end++;assert(c);if(quote){if(escape)escape=0;else if(c=='\\')escape=1;else if(c=='"')quote=0;}else if(c=='"')quote=1;else if(c=='{')depth++;else if(c=='}')depth--;}while(depth);size_t n=(size_t)(end-s);char*r=malloc(n+1);assert(r);memcpy(r,s,n);r[n]=0;return r;}
static char *insert(char*s,size_t at,const char*add){size_t n=strlen(s),m=strlen(add);char*r=malloc(n+m+1);assert(r);memcpy(r,s,at);memcpy(r+at,add,m);memcpy(r+at+m,s+at,n-at+1);free(s);return r;}
static void install(PelePublicDocument*d,uint32_t reference){
    PeleResult*r=pele_public_snapshot_v4(d);assert(!pele_result_status(r));char*all=copy_result(r);pele_result_free(r);
    char*g=object(strstr(all,"\"graph\":")+8);free(all);
    unsigned root=0;assert(sscanf(strstr(g,"\"root\":"),"\"root\":{\"kind\":\"ref\",\"value\":%u}",&root)==1);
    char*strings=strstr(g,"\"strings\":[")+11;char*p=strings;unsigned count=0;while(*p!=']'){assert(*p=='[');count++;p=strchr(p,']')+1;if(*p==',')p++;}
    g=insert(g,(size_t)(p-g),",[103,101,116,84,97,115,107,115]");
    char*node=strstr(g,"\"nodes\":[")+9;for(unsigned i=0;i<root;i++){char*skip=object(node);node+=strlen(skip)+1;free(skip);}char*properties=strstr(node,"\"properties\":[")+14;
    char entry[160];snprintf(entry,sizeof entry,"{\"key\":%u,\"value\":{\"kind\":\"hostReference\",\"value\":%u},\"flags\":7},",count,reference);
    g=insert(g,(size_t)(properties-g),entry);
    good(pele_public_apply_v4(d,0,(const uint8_t*)g,strlen(g)));free(g);
}
static char *access(PeleActiveModelV12*v,const char*request){PeleResult*r=pele_active_model_access_v12(v,(const uint8_t*)request,strlen(request));assert(!pele_result_status(r));char*s=copy_result(r);pele_result_free(r);if(!strstr(s,"\"Ok\"")){fprintf(stderr,"access %s => %s\n",request,s);abort();}char*atom=strstr(s,"\"kind\":\"atom\",\"value\":");assert(atom);char*out=object(atom+strlen("\"kind\":\"atom\",\"value\":"));free(s);return out;}
static char *read_field(PeleActiveModelV12*v,const char*receiver,const char*key){char q[512];snprintf(q,sizeof q,"{\"operation\":\"read\",\"receiver\":%s,\"key\":\"%s\"}",receiver,key);return access(v,q);}
static void set_field(PeleActiveModelV12*v,const char*receiver,const char*key,const char*value){char q[1024];snprintf(q,sizeof q,"{\"operation\":\"set\",\"receiver\":%s,\"key\":\"%s\",\"value\":%s}",receiver,key,value);free(access(v,q));}
struct Context{PelePublicDocument*document;unsigned retains,releases,calls,mode,clock;char*first;char*date;};
static void retain(void*p){((struct Context*)p)->retains++;}
static void release(void*p){((struct Context*)p)->releases++;}
static void release_bytes(void*p){free(p);}
static void invoke(void*p,PeleActiveModelV12*v,const uint8_t*bytes,size_t len,PeleHostFunctionReplyV12*reply){
    struct Context*c=p;c->calls++;char*input=malloc(len+1);assert(input);memcpy(input,bytes,len);input[len]=0;
    char*receiver=object(strstr(input,"\"receiver\":")+11);assert(strstr(input,"4278cc251f400000"));free(input);
    failure(pele_public_snapshot_v4(c->document),"already in use");
    char q[1024];snprintf(q,sizeof q,"{\"operation\":\"invokeBuiltin\",\"call\":{\"method\":19,\"receiver\":%s,\"arguments\":[{\"kind\":\"number\",\"value\":\"4278cc251f400000\"}]}}",receiver);
    char*tasks=access(v,q);char*first=read_field(v,tasks,"0");char*second=read_field(v,tasks,"1");char*end=read_field(v,first,"endTime");char*start=read_field(v,second,"startTime");assert(!strcmp(end,start));
    if(c->first){assert(!strcmp(first,c->first));}else{c->first=strdup(first);c->date=strdup(end);}
    char*label=access(v,"{\"operation\":\"intern\",\"units\":[67,104,97,110,103,101,100,32,108,105,118,101]}");set_field(v,first,"task",label);free(label);
    /* Invalid primitive is caught; later calls still use the same lease. */
    PeleResult*bad=pele_active_model_access_v12(v,(const uint8_t*)"{}",2);failure(bad,"Invalid active model operation");
    if(c->mode==1){char*alternate=access(v,"{\"operation\":\"array\"}");snprintf(q,sizeof q,"{\"operation\":\"push\",\"receiver\":%s,\"value\":%s}",alternate,first);free(access(v,q));free(tasks);tasks=alternate;}
    if(c->mode==2){reply->status=2;reply->exception=77;free(tasks);}else{reply->payload=(PeleForeignBytesV10){tasks,(const uint8_t*)tasks,strlen(tasks),release_bytes};}
    free(receiver);free(first);free(second);free(end);free(start);
}
static uint32_t clock_now(void*p,double*out){((struct Context*)p)->clock++;*out=1704067200000.;return 0;}
static PelePublicDocument*parse(void){const char*source="gantt\ndateFormat YYYY-MM-DD\nsection S\nA:a,2024-01-01,1d\nB:b,after a,2d";PelePublicDocument*d=NULL;good(pele_public_parse_v4((const uint8_t*)source,strlen(source),NULL,0,&d));assert(d);return d;}
static uint32_t bind(PelePublicDocument*d,struct Context*c){PeleHostFunctionV12 table={12,sizeof table,c,retain,release,invoke,{0,0}};uint32_t id=99;good(pele_public_bind_host_function_v12(d,&table,&id));assert(id);return id;}
static PeleResult*render(struct Context*c,uint64_t revision,uint32_t kind,uint64_t bits){PeleHostResourcesV3 host={0};host.base.context=c;host.base.now=clock_now;return pele_public_render_operation_with_functions_v12(c->document,revision,NULL,0,NULL,kind,bits,&host);}
static char*metadata(PeleResult*r){assert(r&&!pele_result_status(r));size_t n=pele_result_len(r);const uint8_t*b=pele_result_data(r);assert(n>24 && !memcmp(b,"PELGBIN1",8));uint32_t length=(uint32_t)b[20]|((uint32_t)b[21]<<8)|((uint32_t)b[22]<<16)|((uint32_t)b[23]<<24);assert(length<n-24);char*s=malloc(length+1);assert(s);memcpy(s,b+n-length,length);s[length]=0;return s;}
int main(void){
    struct Context c={0};c.document=parse();uint32_t id=bind(c.document,&c);assert(c.retains==1);install(c.document,id);
    PeleResult*r=render(&c,1,0,0);char*full=metadata(r);assert(strstr(full,"Changed live"));assert(c.calls==1&&c.clock==1);pele_result_free(r);
    c.mode=1;r=render(&c,2,0,0);char*shorter=metadata(r);assert(strcmp(full,shorter));free(shorter);pele_result_free(r);
    c.mode=2;r=render(&c,3,0,0);char*failed=metadata(r);assert(strstr(failed,"\"hostException\",\"value\":77"));free(failed);pele_result_free(r);
    unsigned calls=c.calls;failure(render(&c,3,0,0),"Stale public model revision");assert(c.calls==calls);
    c.mode=0;r=render(&c,4,0,0);char*recovered=metadata(r);assert(!strcmp(full,recovered));free(recovered);pele_result_free(r);
    r=render(&c,5,2,0);failed=metadata(r);assert(strstr(failed,"\"Err\""));free(failed);pele_result_free(r);
    PeleHostFunctionV12 invalid={12,sizeof invalid,&c,retain,release,invoke,{1,0}};uint32_t out=999;failure(pele_public_bind_host_function_v12(c.document,&invalid,&out),"reserved");assert(out==0&&c.retains==1);
    uint32_t short_header[2]={12,8};out=99;failure(pele_public_bind_host_function_v12(c.document,(const PeleHostFunctionV12*)short_header,&out),"size");assert(out==0);
    struct Context other={0};other.document=parse();uint32_t other_id=bind(other.document,&other);assert(other_id!=id);install(other.document,id);
    r=render(&other,1,0,0);failed=metadata(r);assert(strstr(failed,"foreign-owner"));assert(!other.calls);free(failed);pele_result_free(r);
    pele_public_document_free_v4(other.document);assert(other.releases==1);
    pele_public_document_free_v4(c.document);assert(c.releases==1);assert(strstr(full,"Changed live"));
    free(full);free(c.first);free(c.date);puts("Gantt live host method C v12 integration PASS");return 0;
}
