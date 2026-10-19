#include "pele.h"
#include <assert.h>
#include <math.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
_Static_assert(sizeof(PelePreparedOptionsV13)==48,"prepared packet ABI");
_Static_assert(offsetof(PelePreparedOptionsV13,max_edges_bits)==24,"prepared bits ABI");
static uint64_t bits(double value) { uint64_t out;memcpy(&out,&value,8);return out; }
static char *text(PeleResult *result) {
    assert(result);size_t len=pele_result_len(result);assert(len<=64*1024*1024);
    char *out=malloc(len+1);assert(out);memcpy(out,pele_result_data(result),len);out[len]=0;return out;
}
static void okay(PeleResult *result) { assert(result && pele_result_status(result)==0);pele_result_free(result); }
static void error(PeleResult *result,const char *message) {
    assert(result);char *body=text(result);
    if(pele_result_status(result)==0)fprintf(stderr,"Expected failure %s, got success %.1024s\n",message,body);
    assert(pele_result_status(result)!=0);
    if(!strstr(body,message))fprintf(stderr,"Expected %s, got %.1024s\n",message,body);
    assert(strstr(body,message));free(body);pele_result_free(result);
}
static PeleResult *parse(const char *kind,const char *source,const PelePreparedOptionsV13 *options,PelePublicDocument **out) {
    return pele_parse_prepared_v13((const uint8_t *)kind,strlen(kind),(const uint8_t *)source,strlen(source),NULL,0,NULL,0,options,out);
}
static uint64_t le64(const uint8_t *p) { uint64_t n=0;for(unsigned i=0;i<8;i++)n|=(uint64_t)p[i]<<(8*i);return n; }
static char *metadata(PeleResult *result,uint64_t revision) {
    assert(result && pele_result_status(result)==0);const uint8_t *bytes=pele_result_data(result);size_t len=pele_result_len(result);
    assert(len>=24 && !memcmp(bytes,"PELGBIN1",8) && le64(bytes+12)==revision);
    size_t count=(size_t)bytes[20]|(size_t)bytes[21]<<8|(size_t)bytes[22]<<16|(size_t)bytes[23]<<24;
    assert(count<=len-24);char *out=malloc(count+1);assert(out);memcpy(out,bytes+len-count,count);out[count]=0;return out;
}
struct Host { unsigned calls;int fail;PelePublicDocument *document; };
static uint32_t width(void *context,const uint8_t *s,size_t n,double size,uint32_t style,const uint8_t *font,size_t font_len,const uint8_t *mono,size_t mono_len,double *out) {
    (void)s;(void)style;(void)font;(void)font_len;(void)mono;(void)mono_len;struct Host *host=context;host->calls++;
    if(host->document)error(pele_public_snapshot_binary_v5(host->document),"already in use");
    if(host->fail)return 1;*out=n*size*0.6;return 0;
}
int main(void) {
    const char *source="flowchart LR\nA[One] --> B[Two]\n";PelePublicDocument *document=NULL;
    PelePreparedOptionsV13 options={13,sizeof options,0,0,0,0,{0,0}};
    okay(parse("flowchart",source,&options,&document));assert(document);
    okay(pele_public_snapshot_binary_v5(document));okay(pele_public_schema_v4(document));
    okay(pele_prepared_schema_v13((const uint8_t *)"flowchart",9));
    error(pele_prepared_schema_v13((const uint8_t *)"unknown",7),"not supported");
    PeleResult *snapshot=pele_public_snapshot_binary_v5(document);assert(pele_result_status(snapshot)==0);
    const uint8_t *raw=pele_result_data(snapshot);size_t total=pele_result_len(snapshot);
    size_t metadata_count=(size_t)raw[20]|(size_t)raw[21]<<8|(size_t)raw[22]<<16|(size_t)raw[23]<<24;
    assert(metadata_count<=total-24);size_t graph_len=total-metadata_count;
    uint8_t *graph=malloc(graph_len);assert(graph);memcpy(graph,raw,graph_len);memset(graph+20,0,4);
    PelePublicDocument *fresh=NULL;
    okay(pele_create_prepared_v13((const uint8_t *)"flowchart",9,graph,graph_len,NULL,0,&fresh));assert(fresh);
    graph[12]=1;
    PelePublicDocument *bad=(PelePublicDocument *)(uintptr_t)1;
    error(pele_create_prepared_v13((const uint8_t *)"flowchart",9,graph,graph_len,NULL,0,&bad),"revision zero");assert(!bad);
    bad=(PelePublicDocument *)(uintptr_t)1;
    error(pele_create_prepared_v13((const uint8_t *)"flowchart",9,NULL,graph_len,NULL,0,&bad),"input is null");assert(!bad);
    error(pele_create_prepared_v13(NULL,1025,(const uint8_t *)(uintptr_t)1,graph_len,NULL,0,&bad),"declared input exceeds budget");assert(!bad);
    memset(graph,0,graph_len);free(graph);pele_result_free(snapshot);
    PeleResult *fresh_drawing=pele_render_prepared_v13(fresh,0,NULL,0,NULL,0,NULL,NULL);
    char *fresh_body=metadata(fresh_drawing,1);assert(strstr(fresh_body,"One"));free(fresh_body);pele_result_free(fresh_drawing);pele_public_document_free_v4(fresh);
    struct Host calls={0};calls.document=document;PeleHostResourcesV3 host={0};host.base.context=&calls;host.base.width=width;
    const char *settings="{\"outputLimit\":0}";
    PeleResult *retained=pele_render_prepared_v13(document,0,NULL,0,(const uint8_t *)settings,strlen(settings),NULL,&host);
    char *body=metadata(retained,1);assert(strstr(body,"\"Ok\":{"));assert(strstr(body,"One"));free(body);assert(calls.calls>0);
    calls.fail=1;PeleResult *failed=pele_render_prepared_v13(document,1,NULL,0,NULL,0,NULL,&host);
    body=metadata(failed,2);assert(strstr(body,"Host text measurer failed"));free(body);pele_result_free(failed);calls.fail=0;
    PeleRenderNumbersV9 numbers={9,sizeof numbers,PELE_RENDER_NUMBER_PADDING_V9,0,0,0,0,0};numbers.padding_bits=bits(NAN);
    PeleResult *nonfinite=pele_render_prepared_v13(document,2,NULL,0,NULL,0,&numbers,&host);
    body=metadata(nonfinite,3);assert(strstr(body,"\"$numbers\":{"));free(body);pele_result_free(nonfinite);
    error(pele_render_prepared_v13(document,2,NULL,0,NULL,0,NULL,&host),"Stale public model revision");
    calls.document=NULL;pele_public_document_free_v4(document);body=metadata(retained,1);assert(strstr(body,"One"));free(body);pele_result_free(retained);
    for(unsigned i=0;i<4;i++) {
        options=(PelePreparedOptionsV13){13,sizeof options,0,0,0,0,{0,0}};
        if(i==0)options.version=12;if(i==1)options.struct_size=8;if(i==2)options.title_present=2;if(i==3)options.max_edges_kind=3;
        document=(PelePublicDocument *)(uintptr_t)1;
        error(parse("flowchart",source,&options,&document),i<2?"version or size":i==2?"flags or reserved":"edge policy");assert(!document);
    }
    document=(PelePublicDocument *)(uintptr_t)1;
    error(pele_parse_prepared_v13(NULL,0,(const uint8_t *)(uintptr_t)1,16*1024*1024+1,NULL,0,NULL,0,(const PelePreparedOptionsV13 *)(uintptr_t)1,&document),"declared input exceeds budget");assert(!document);
    okay(parse("pie","pie\n\"One &amp; Two\":1\n",NULL,&document));
    PeleResult *verbatim=pele_render_prepared_v13(document,0,NULL,0,NULL,0,NULL,NULL);
    body=metadata(verbatim,1);assert(strstr(body,"\"Ok\":"));free(body);pele_result_free(verbatim);pele_public_document_free_v4(document);document=NULL;
    options=(PelePreparedOptionsV13){13,sizeof options,UINT64_MAX,0,0,0,{0,0}};
    error(parse("pie","pie\n\"One\":1\n",&options,&document),"position");assert(!document);
    options=(PelePreparedOptionsV13){13,sizeof options,0,2,0,0,{0,0}};
    error(parse("flowchart",source,&options,&document),"limit");assert(!document);
    error(parse("unregistered",source,NULL,&document),"unregistered");assert(!document);
    puts("C13 prepared/raw integration passed: ownership, packet/policy, raw models, IEEE values and revisions");return 0;
}
