#include "pele.h"
#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
_Static_assert(sizeof(double) == 8, "binary64 host ABI");
_Static_assert(sizeof(PeleRenderNumbersV9) == 56, "v9 packet layout");
_Static_assert(offsetof(PeleRenderNumbersV9, present) == 8, "v9 mask offset");
_Static_assert(offsetof(PeleRenderNumbersV9, now_bits) == 48, "v9 final slot");
static uint64_t bits(double value) { uint64_t out; memcpy(&out, &value, 8); return out; }
static char *text(PeleResult *result) { size_t n=pele_result_len(result); char *out=malloc(n+1); assert(out); memcpy(out,pele_result_data(result),n);out[n]=0;return out; }
static void failure(PeleResult *result, uint32_t status, const char *message) { assert(result && pele_result_status(result)==status);char *body=text(result);assert(strstr(body,message));free(body);pele_result_free(result); }
static void success(PeleResult *result) { assert(result && pele_result_status(result)==0);pele_result_free(result); }
static void outcome(PeleResult *result, uint64_t revision, const char *message) {
    assert(result && pele_result_status(result)==0);const uint8_t *p=pele_result_data(result);size_t n=pele_result_len(result);assert(n>=24&&!memcmp(p,"PELGBIN1",8));
    uint64_t actual=0;for(unsigned i=0;i<8;i++)actual|=(uint64_t)p[12+i]<<(8*i);assert(actual==revision);
    size_t m=(size_t)p[20]|(size_t)p[21]<<8|(size_t)p[22]<<16|(size_t)p[23]<<24;assert(m<=n-24);
    char *body=malloc(m+1);assert(body);memcpy(body,p+n-m,m);body[m]=0;assert(strstr(body,message));free(body);pele_result_free(result);
}
struct Host {unsigned widths,clocks;int fail_width,fail_clock;};
static uint32_t width(void *raw,const uint8_t *s,size_t n,double size,uint32_t style,const uint8_t *f,size_t fn,const uint8_t *m,size_t mn,double *out) {
    (void)s;(void)f;(void)fn;(void)m;(void)mn;struct Host *host=raw;host->widths++;if(host->fail_width)return 1;*out=(double)n*size*((style&1)?0.71:0.6);return 0;
}
static uint32_t clock_now(void *raw,double *out) {struct Host *host=raw;host->clocks++;if(host->fail_clock)return 1;*out=1704067200000.5;return 0;}
static PeleResult *render(const char *source,const char *json,const PeleRenderNumbersV9 *packet,uint32_t kind,uint64_t number,const PeleHostResourcesV3 *host) {
    return pele_render_with_numbers_v9((const uint8_t *)source,strlen(source),(const uint8_t *)json,json?strlen(json):0,packet,kind,number,host);
}
int main(void) {
    const char *source="pie title Native\n\"A\":1\n\"B\":2";
    PeleRenderNumbersV9 packet={9,56,PELE_RENDER_NUMBER_PADDING_V9,0,0,0,0,0};packet.padding_bits=bits(3.5);
    success(render(source,"{\"padding\":\"ignored\"}",&packet,3,0,NULL));
    failure(render(source,"{\"padding\":\"ignored\"}",NULL,3,0,NULL),1,"Invalid render options");
    failure(render(source,"{\"fontSize\":\"bad\"}",&packet,3,0,NULL),1,"Invalid render options");
    failure(render(source,"{\"limit\":-1}",&packet,3,0,NULL),1,"Invalid render options");
    failure(render(source,"{\"nativeTheme\":{\"background\":[]}}",&packet,3,0,NULL),1,"Invalid render options");
    failure(render(source,"{\"padding\":{},\"padding\":3}",&packet,3,0,NULL),1,"duplicate field");
    failure(render(source,"{\"padding\":null} false",&packet,3,0,NULL),1,"Invalid render options");
    failure(render(source,"{\"padding\": [}",&packet,3,0,NULL),1,"Invalid render options");
    failure(render(source,"{\"outputLimit\":\"bad\"}",&packet,3,0,NULL),1,"Invalid render options");
    success(render(source,"{\"outputLimit\":\"ignored\"}",&packet,0,0,NULL));
    failure(render(source,"{\"outputLimit\":0,\"outputLimit\":1}",&packet,0,0,NULL),1,"duplicate field");
    packet.present=31;packet.font_size_bits=bits(13.25);packet.max_width_bits=bits(220.5);packet.direction_breakpoint_bits=bits(80.25);packet.padding_bits=bits(-0.0);packet.now_bits=UINT64_C(0xfff8000000000042);
    success(render(source,"{\"fontSize\":\"ignored\",\"maxWidth\":[],\"directionBreakpoint\":{},\"padding\":null,\"now\":false}",&packet,0,0,NULL));
    assert(packet.font_size_bits==bits(13.25)&&packet.max_width_bits==bits(220.5)&&packet.direction_breakpoint_bits==bits(80.25)&&packet.padding_bits==UINT64_C(0x8000000000000000)&&packet.now_bits==UINT64_C(0xfff8000000000042));
    packet.present=0;packet.font_size_bits=UINT64_C(0x7ff8000000000042);packet.now_bits=UINT64_C(0xfff0000000000000);
    for(unsigned i=0;i<8;i++) {
        PeleResult *old=pele_render_with_resources_v3((const uint8_t *)source,strlen(source),NULL,0,NULL);
        PeleResult *current=render(source,NULL,&packet,3,0,NULL);assert(pele_result_status(old)==0&&pele_result_status(current)==0);
        assert(pele_result_len(old)==pele_result_len(current)&&!memcmp(pele_result_data(old),pele_result_data(current),pele_result_len(old)));
        pele_result_free(old);pele_result_free(current);
    }
    PeleRenderNumbersV9 invalid_version=packet;invalid_version.version=8;
    failure(render(source,NULL,&invalid_version,3,0,NULL),1,"unsupported version");
    struct {uint32_t version,size;} header={9,8};
    failure(render(source,NULL,(const PeleRenderNumbersV9 *)&header,3,0,NULL),1,"structure size");header.size=64;
    failure(render(source,NULL,(const PeleRenderNumbersV9 *)&header,3,0,NULL),1,"structure size");
    packet.present=32;failure(render(source,NULL,&packet,3,0,NULL),1,"numeric mask");packet.present=0;
    failure(render(source,NULL,&packet,99,0,NULL),1,"unknown output-limit kind");
    failure(pele_render_with_numbers_v9((const uint8_t *)(uintptr_t)1,16*1024*1024+1,(const uint8_t *)(uintptr_t)1,1024*1024+1,(const PeleRenderNumbersV9 *)(uintptr_t)1,99,0,NULL),2,"declared input exceeds budget");
    failure(pele_render_with_numbers_v9((const uint8_t *)(uintptr_t)1,16*1024*1024+1,NULL,0,(const PeleRenderNumbersV9 *)(uintptr_t)1,99,0,NULL),2,"declared input exceeds budget");
    failure(pele_render_document_with_numbers_v9(NULL,(const uint8_t *)(uintptr_t)1,1024*1024+1,(const PeleRenderNumbersV9 *)(uintptr_t)1,99,0,NULL),2,"declared input exceeds budget");
    failure(pele_render_with_numbers_v9(NULL,1,NULL,0,&packet,3,0,NULL),1,"null input");
    failure(pele_render_with_numbers_v9(NULL,1,(const uint8_t *)"{",1,&packet,3,0,NULL),1,"Invalid render options");
    failure(pele_render_document_with_numbers_v9(NULL,NULL,0,&packet,3,0,NULL),1,"parse() returned");
    PeleResult *document=pele_parse_v2((const uint8_t *)source,strlen(source),NULL,0);assert(pele_result_status(document)==0);
    success(pele_render_document_with_numbers_v9(document,NULL,0,NULL,0,0,NULL));pele_result_free(document);
    struct Host counters={0};PeleHostResourcesV3 host={0};host.base.context=&counters;host.base.width=width;host.base.now=clock_now;
    packet.present=PELE_RENDER_NUMBER_FONT_SIZE_V9;packet.font_size_bits=UINT64_C(0x7ff8000000000042);
    PeleResult *bad_source=render("flowchart TD\nA --> [",NULL,&packet,0,0,&host);assert(pele_result_status(bad_source)==1);pele_result_free(bad_source);assert(counters.clocks==0&&counters.widths==0);
    const char *gantt="gantt\ndateFormat YYYY-MM-DD\nsection S\nA :a,2024-01-01,1d";
    packet.present=PELE_RENDER_NUMBER_NOW_V9;packet.now_bits=UINT64_C(0x7ff8000000000042);
    success(render(gantt,NULL,&packet,0,0,&host));assert(counters.clocks==1);
    counters.clocks=0;packet.now_bits=UINT64_C(0x8000000000000000);success(render(gantt,NULL,&packet,0,0,&host));assert(counters.clocks==0);
    packet.now_bits=UINT64_C(0x7ff0000000000000);counters.clocks=0;success(render(source,NULL,&packet,0,0,&host));assert(counters.clocks==0);
    counters.widths=0;failure(render("classDiagram\nclass LongIdentifier","{\"outputLimit\":0}",&packet,3,0,&host),2,"output is longer");assert(counters.widths==0);
    failure(render("classDiagram\nclass LongIdentifier","{\"outputLimit\":0}",&packet,2,bits(0),&host),2,"output is longer");assert(counters.widths>0);
    PelePublicDocument *mutable=NULL;success(pele_public_parse_v4((const uint8_t *)gantt,strlen(gantt),NULL,0,&mutable));
    packet.present=32;counters.clocks=0;counters.widths=0;
    failure(pele_public_render_operation_with_numbers_v9(mutable,0,NULL,0,&packet,3,0,&host),1,"numeric mask");assert(counters.clocks==0&&counters.widths==0);
    outcome(pele_public_snapshot_binary_v5(mutable),0,"\"type\":\"gantt\"");packet.present=PELE_RENDER_NUMBER_NOW_V9;
    counters.fail_clock=1;outcome(pele_public_render_operation_with_numbers_v9(mutable,0,NULL,0,&packet,0,0,&host),1,"Host clock failed");
    counters.fail_clock=0;counters.fail_width=1;outcome(pele_public_render_operation_with_numbers_v9(mutable,1,NULL,0,&packet,0,0,&host),2,"Host text measurer failed");
    counters.fail_width=0;outcome(pele_public_render_operation_with_numbers_v9(mutable,2,NULL,0,&packet,2,bits(0),&host),3,"limit of 0 characters");
    counters.clocks=0;counters.widths=0;
    failure(pele_public_render_operation_with_numbers_v9(mutable,2,NULL,0,&packet,0,0,&host),1,"Stale public model revision");assert(counters.clocks==0&&counters.widths==0);
    outcome(pele_public_render_operation_with_numbers_v9(mutable,3,NULL,0,&packet,0,0,&host),4,"\"Ok\":");pele_public_document_free_v4(mutable);
    /* A valid <=16MiB source can produce >64MiB of JSON error escaping. */
    const size_t nul_count=6*1024*1024;uint8_t *nul_source=malloc(nul_count+4);assert(nul_source);
    memcpy(nul_source,"pie\n",4);memset(nul_source+4,0,nul_count);
    const char *large_options="{\"limit\":16777216}";counters.widths=0;counters.clocks=0;
    PeleResult *large_error=pele_render_with_numbers_v9(nul_source,nul_count+4,(const uint8_t *)large_options,strlen(large_options),NULL,3,0,&host);
    assert(pele_result_len(large_error)<=64*1024*1024&&counters.widths==0&&counters.clocks==0);
    failure(large_error,2,"result exceeds 64 MiB JSON budget");free(nul_source);
    puts("Native render-number C integration passed: packet, filtered options, precedence, clocks, ownership and revisions");return 0;
}
