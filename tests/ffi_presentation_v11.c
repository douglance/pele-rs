#include "pele.h"
#include <assert.h>
#include <math.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

_Static_assert(sizeof(PelePresentationOptionsV11)==40,"v11 presentation packet ABI");
_Static_assert(offsetof(PelePresentationOptionsV11,max_elements)==8,"v11 budget offset");
_Static_assert(offsetof(PelePresentationOptionsV11,reserved)==24,"v11 reserved offset");
static const char *flow="flowchart LR\naccTitle: Native links\naccDescr: Owned accessible frame\nA[Start] -->|next| B[Finish]\nclick A \"https://example.test/a\"";
static const char *gantt="gantt\ndateFormat YYYY-MM-DD\nsection S\nA :a,2024-01-01,1d";
static uint64_t bits(double value) { uint64_t result;memcpy(&result,&value,8);return result; }
static uint32_t le32(const uint8_t *p) { return (uint32_t)p[0]|(uint32_t)p[1]<<8|(uint32_t)p[2]<<16|(uint32_t)p[3]<<24; }
static uint64_t le64(const uint8_t *p) { uint64_t value=0;for(unsigned i=0;i<8;i++)value|=(uint64_t)p[i]<<(8*i);return value; }
static char *copy(const uint8_t *bytes,size_t size) { char *s=malloc(size+1);assert(s);memcpy(s,bytes,size);s[size]=0;return s; }
static char *text(PeleResult *result) { assert(result);assert(pele_result_len(result)<=64*1024*1024);return copy(pele_result_data(result),pele_result_len(result)); }
static void success(PeleResult *result) { assert(result && pele_result_status(result)==0); }
static void release_success(PeleResult *result) { success(result);pele_result_free(result); }
static void failure(PeleResult *result,uint32_t status,const char *expected) {
    assert(result && pele_result_status(result)==status);char *s=text(result);assert(strstr(s,expected));free(s);pele_result_free(result);
}
static char *metadata(PeleResult *result,uint64_t revision) {
    success(result);const uint8_t *bytes=pele_result_data(result);size_t count=pele_result_len(result);
    assert(count>=24 && !memcmp(bytes,"PELGBIN1",8) && le32(bytes+8)==1 && le64(bytes+12)==revision);
    size_t length=le32(bytes+20);assert(length<=count-24);return copy(bytes+count-length,length);
}
static void outcome(PeleResult *result,uint64_t revision,const char *expected,int presentation) {
    char *s=metadata(result,revision);
    if(!strstr(s,expected))fprintf(stderr,"revision %llu expected %s; metadata %.2048s\n",(unsigned long long)revision,expected,s);
    assert(strstr(s,expected));
    assert((strstr(s,"\"presentation\":")!=NULL)==presentation);free(s);pele_result_free(result);
}
/* Presentation is the final additive member of the existing rendered result.
 * Removing only that member must recover the complete prior wire drawing. */
static void same_drawing(const char *presented,const char *ordinary) {
    const char *member=strstr(presented,",\"presentation\":");assert(member);
    size_t prefix=(size_t)(member-presented);assert(strlen(ordinary)==prefix+2);
    assert(!memcmp(presented,ordinary,prefix));assert(!strcmp(ordinary+prefix,"}}"));
}
static void available(const char *body) {
    assert(strstr(body,"\"presentation\":{\"status\":\"available\",\"value\":{\"version\":1"));
    assert(strstr(body,"\"title\":\"Native links\""));assert(strstr(body,"\"description\":\"Owned accessible frame\""));
    assert(strstr(body,"\"text\":\"Start\""));assert(strstr(body,"\"text\":\"next\""));
    const char *bounds=strstr(body,"\"bounds\":{");assert(bounds);
    double x,y,w,h;assert(sscanf(bounds,"\"bounds\":{\"x\":%lf,\"y\":%lf,\"width\":%lf,\"height\":%lf",&x,&y,&w,&h)==4);
    assert(isfinite(x)&&isfinite(y)&&isfinite(w)&&isfinite(h)&&w>0&&h>0);
    assert(strstr(body,"\"href\":\"https://example.test/a\",\"internal\":false,\"bounds\":"));
}
struct Host { unsigned widths,clocks;int fail;PelePublicDocument *active; };
static uint32_t width(void *raw,const uint8_t *s,size_t n,double size,uint32_t style,
    const uint8_t *font,size_t font_len,const uint8_t *mono,size_t mono_len,double *out) {
    (void)s;(void)font;(void)font_len;(void)mono;(void)mono_len;struct Host *host=raw;host->widths++;
    if(host->active) failure(pele_public_snapshot_binary_v5(host->active),1,"already in use");
    if(host->fail)return 1;*out=n*size*((style&1)?0.71:0.6);return 0;
}
static uint32_t clock_now(void *raw,double *out) { ((struct Host *)raw)->clocks++;*out=1704067200000.5;return 0; }
static PeleResult *source(const char *input,const char *options,const PeleRenderNumbersV9 *numbers,
    uint32_t kind,uint64_t limit,const PelePresentationOptionsV11 *packet,const PeleHostResourcesV3 *host) {
    return pele_render_presentation_v11((const uint8_t *)input,strlen(input),(const uint8_t *)options,options?strlen(options):0,numbers,kind,limit,packet,host);
}
static PelePublicDocument *parsed(const char *input) {
    PelePublicDocument *document=NULL;release_success(pele_public_parse_v4((const uint8_t *)input,strlen(input),NULL,0,&document));assert(document);return document;
}
static void compare_source(const char *input,const char *options,const PeleRenderNumbersV9 *numbers,
    uint32_t kind,uint64_t limit,const PelePresentationOptionsV11 *packet,const char *state,const PeleHostResourcesV3 *host) {
    PeleResult *current=source(input,options,numbers,kind,limit,packet,host);
    PeleResult *old=pele_render_with_numbers_v9((const uint8_t *)input,strlen(input),(const uint8_t *)options,options?strlen(options):0,numbers,kind,limit,host);
    success(current);success(old);char *a=text(current),*b=text(old);same_drawing(a,b);assert(strstr(a,state));
    free(a);free(b);pele_result_free(current);pele_result_free(old);
}
static void three_owned_lanes(const PeleHostResourcesV3 *host) {
    PeleResult *current=source(flow,NULL,NULL,0,0,NULL,host);success(current);char *a=text(current);available(a);
    PeleResult *ordinary=pele_render_with_numbers_v9((const uint8_t *)flow,strlen(flow),NULL,0,NULL,0,0,host);success(ordinary);char *b=text(ordinary);same_drawing(a,b);
    free(b);pele_result_free(ordinary);free(a);pele_result_free(current);
    PeleResult *document=pele_parse_v2((const uint8_t *)flow,strlen(flow),NULL,0);success(document);
    current=pele_render_document_presentation_v11(document,NULL,0,NULL,0,0,NULL,host);
    ordinary=pele_render_document_with_numbers_v9(document,NULL,0,NULL,0,0,host);success(current);success(ordinary);
    pele_result_free(document);a=text(current);b=text(ordinary);available(a);same_drawing(a,b);
    free(a);free(b);pele_result_free(current);pele_result_free(ordinary);
    PelePublicDocument *first=parsed(flow),*second=parsed(flow);
    current=pele_public_render_presentation_v11(first,0,NULL,0,NULL,0,0,NULL,host);
    ordinary=pele_public_render_operation_with_numbers_v9(second,0,NULL,0,NULL,0,0,host);
    a=metadata(current,1);b=metadata(ordinary,1);same_drawing(a,b);available(a);
    size_t ca=pele_result_len(current)-le32(pele_result_data(current)+20),cb=pele_result_len(ordinary)-le32(pele_result_data(ordinary)+20);
    assert(ca==cb && !memcmp(pele_result_data(current),pele_result_data(ordinary),20));
    assert(!memcmp(pele_result_data(current)+24,pele_result_data(ordinary)+24,ca-24));
    pele_public_document_free_v4(first);pele_public_document_free_v4(second);
    assert(strstr(a,"Native links"));free(a);free(b);pele_result_free(current);pele_result_free(ordinary);
}
static void packet_preflight(struct Host *calls,const PeleHostResourcesV3 *host) {
    PeleResult *typed=pele_parse_v2((const uint8_t *)flow,strlen(flow),NULL,0);success(typed);PelePublicDocument *document=parsed(flow);
    PelePresentationOptionsV11 invalid={11,sizeof(PelePresentationOptionsV11),4096,1024*1024,{0,0}};
    for(unsigned variant=0;variant<3;variant++) {
        invalid.version=variant==0?10:11;invalid.struct_size=variant==1?8:sizeof invalid;invalid.reserved[1]=variant==2?1:0;
        calls->widths=0;calls->clocks=0;const char *expected=variant==2?"reserved fields":"version or size";
        failure(source(flow,"{",NULL,0,0,&invalid,host),1,expected);
        failure(pele_render_document_presentation_v11(typed,(const uint8_t *)"{",1,NULL,0,0,&invalid,host),1,expected);
        failure(pele_public_render_presentation_v11(document,0,(const uint8_t *)"{",1,NULL,0,0,&invalid,host),1,expected);
        assert(calls->widths==0 && calls->clocks==0);
        outcome(pele_public_snapshot_binary_v5(document),0,"\"type\":\"flowchart\"",0);
    }
    struct {uint32_t version,size;} short_header={11,8};
    failure(source(flow,NULL,NULL,0,0,(const PelePresentationOptionsV11 *)&short_header,host),1,"version or size");
    invalid=(PelePresentationOptionsV11){11,sizeof invalid,UINT64_MAX,UINT64_MAX,{0,0}};
    uint8_t unaligned[sizeof invalid+1];memcpy(unaligned+1,&invalid,sizeof invalid);
    compare_source(flow,NULL,NULL,0,0,(const PelePresentationOptionsV11 *)(unaligned+1),"\"status\":\"available\"",host);
    failure(pele_render_presentation_v11((const uint8_t *)(uintptr_t)1,16*1024*1024+1,NULL,0,NULL,0,0,(const PelePresentationOptionsV11 *)(uintptr_t)1,host),2,"declared input exceeds budget");
    failure(pele_render_document_presentation_v11(NULL,NULL,0,NULL,0,0,NULL,host),1,"parse() returned");
    failure(pele_public_render_presentation_v11(NULL,0,NULL,0,NULL,0,0,NULL,host),1,"null document");
    pele_result_free(typed);pele_public_document_free_v4(document);
}
static void budgets_numbers_and_dispatch(struct Host *calls,const PeleHostResourcesV3 *host) {
    PelePresentationOptionsV11 budget={11,sizeof budget,0,1024*1024,{0,0}};
    compare_source(flow,NULL,NULL,0,0,&budget,"\"status\":\"unavailable\",\"value\":\"budgetExceeded\"",host);
    budget.max_elements=4096;budget.max_text_bytes=1;
    compare_source(flow,NULL,NULL,0,0,&budget,"\"value\":\"budgetExceeded\"",host);
    compare_source("pie\n\"A\":1",NULL,NULL,0,0,NULL,"\"value\":\"unsupportedDiagram\"",host);
    PeleRenderNumbersV9 numbers={9,sizeof numbers,PELE_RENDER_NUMBER_PADDING_V9,0,0,0,0,0};numbers.padding_bits=bits(NAN);
    compare_source(flow,"{\"padding\":\"overridden\"}",&numbers,2,bits(INFINITY),NULL,"\"value\":\"invalidGeometry\"",host);
    PeleResult *nan=source(flow,NULL,&numbers,2,bits(NAN),NULL,host);success(nan);char *body=text(nan);assert(strstr(body,"\"$numbers\":{"));free(body);pele_result_free(nan);
    failure(source(flow,"{\"fontSize\":\"bad\"}",&numbers,0,0,NULL,host),1,"Invalid render options");
    failure(source(flow,"{\"padding\":0,\"padding\":1}",&numbers,0,0,NULL,host),1,"duplicate field");
    numbers.present=32;failure(source(flow,NULL,&numbers,0,0,NULL,host),1,"numeric mask");numbers.present=0;
    calls->widths=0;failure(source("classDiagram\nclass LongIdentifier","{\"outputLimit\":0}",&numbers,3,0,NULL,host),2,"output is longer");assert(calls->widths==0);
    failure(source("classDiagram\nclass LongIdentifier","{\"outputLimit\":0}",&numbers,2,bits(0),NULL,host),2,"output is longer");assert(calls->widths>0);
    numbers.present=PELE_RENDER_NUMBER_NOW_V9;numbers.now_bits=bits(NAN);calls->clocks=0;
    release_success(source(gantt,NULL,&numbers,0,0,NULL,host));assert(calls->clocks==1);
    numbers.now_bits=bits(-0.0);calls->clocks=0;release_success(source(gantt,NULL,&numbers,0,0,NULL,host));assert(calls->clocks==0);
}
static void mutable_errors_retain_owned_results(struct Host *calls,const PeleHostResourcesV3 *host) {
    PelePublicDocument *document=parsed(flow);calls->active=document;
    PeleResult *good=pele_public_render_presentation_v11(document,0,NULL,0,NULL,0,0,NULL,host);
    char *retained=metadata(good,1);available(retained);
    calls->widths=0;outcome(pele_public_render_presentation_v11(document,1,NULL,0,NULL,2,bits(0),NULL,host),2,"limit of 0 characters",0);assert(calls->widths>0);
    calls->fail=1;outcome(pele_public_render_presentation_v11(document,2,NULL,0,NULL,0,0,NULL,host),3,"Host text measurer failed",0);calls->fail=0;
    const char *theme="{\"nativeTheme\":{\"accent\":\"invalid\"}}";
    outcome(pele_public_render_presentation_v11(document,3,(const uint8_t *)theme,strlen(theme),NULL,0,0,NULL,host),4,"Native theme colors must be #RRGGBB",0);
    calls->widths=0;failure(pele_public_render_presentation_v11(document,3,NULL,0,NULL,0,0,NULL,host),1,"Stale public model revision");assert(calls->widths==0);
    outcome(pele_public_snapshot_binary_v5(document),4,"\"type\":\"flowchart\"",0);
    PelePresentationOptionsV11 zero={11,sizeof zero,0,0,{0,0}};
    outcome(pele_public_render_presentation_v11(document,4,NULL,0,NULL,0,0,&zero,host),5,"\"value\":\"budgetExceeded\"",1);
    PeleResult *recovered=pele_public_render_presentation_v11(document,5,NULL,0,NULL,0,0,NULL,host);
    char *after=metadata(recovered,6);available(after);assert(!strcmp(retained,after));
    char *still=metadata(good,1);assert(!strcmp(still,retained));free(still);free(after);free(retained);
    calls->active=NULL;pele_public_document_free_v4(document);pele_result_free(good);pele_result_free(recovered);
}
int main(void) {
    struct Host calls={0};PeleHostResourcesV3 host={0};host.base.context=&calls;host.base.width=width;host.base.now=clock_now;
    three_owned_lanes(&host);packet_preflight(&calls,&host);budgets_numbers_and_dispatch(&calls,&host);mutable_errors_retain_owned_results(&calls,&host);
    puts("C11 presentation integration passed: three lanes, ownership, packet preflight, budgets, numbers, callbacks and revisions");return 0;
}
