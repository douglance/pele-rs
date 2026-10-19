#include "pele.h"
#include <assert.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct { int refs, icons, clocks, calendars, dates, instants, nested; PeleForeignCallV10 *call; } State;
static uint64_t bits(double value) { uint64_t result; memcpy(&result,&value,8); return result; }
static int contains(const PeleResult *result,const char *text) {
    size_t count=pele_result_len(result);char *copy=malloc(count+1);assert(copy);
    memcpy(copy,pele_result_data(result),count);copy[count]=0;int found=strstr(copy,text)!=NULL;free(copy);return found;
}
static void success(PeleResult *result) { assert(result && pele_result_status(result)==0);pele_result_free(result); }
static void failure(PeleResult *result,const char *message) { assert(result && pele_result_status(result)!=0 && contains(result,message));pele_result_free(result); }
static void release_bytes(void *value) { free(value); }
static PeleForeignBytesV10 owned(const char *value) {
    size_t count=strlen(value);char *copy=malloc(count+1);assert(copy);memcpy(copy,value,count+1);
    return (PeleForeignBytesV10){copy,(uint8_t *)copy,count,release_bytes};
}
static void retain(void *value) { ((State *)value)->refs++; }
static void release(void *value) { ((State *)value)->refs--; }
static void release_model(void *value,void *model) { assert(value==model); }
static void describe(void *value,PeleForeignCallV10 *call,uint32_t phase,PeleForeignReplyV10 *reply) {
    (void)value;(void)phase;reply->payload=owned("{}");
    uint64_t output=123;
    failure(pele_foreign_now_millis_v10(call,&output),"unavailable outside render");assert(output==0);
}
static void parse(void *value,PeleForeignCallV10 *call,const PeleProviderParseV10 *request,PeleForeignReplyV10 *reply) {
    (void)request;reply->model=value;
    failure(pele_foreign_icon_v10(call,(const uint8_t *)"x",1),"unavailable outside render");
}
static uint32_t icon(void *value,const uint8_t *name,size_t count,PeleBorrowedBytes *output) {
    State *state=value;state->icons++;assert(count==1 && name[0]=='A');
    uint64_t number=123;uint32_t present=123;
    failure(pele_foreign_now_millis_v10(state->call,&number),"already in use");assert(number==0);
    failure(pele_foreign_text_width_v10(state->call,(const uint8_t *)"x",1,bits(13),0,&number),"already in use");assert(number==0);
    failure(pele_foreign_icon_v10(state->call,(const uint8_t *)"x",1),"already in use");
    failure(pele_foreign_parse_date_v10(state->call,(const uint8_t *)"x",1,&present,&number),"already in use");assert(present==0 && number==0);
    failure(pele_foreign_calendar_fields_v10(state->call,bits(0)),"already in use");
    const char *civil="{\"year\":2024,\"month\":1,\"day\":2,\"hour\":0,\"minute\":0,\"second\":0,\"millisecond\":0}";
    failure(pele_foreign_calendar_instant_v10(state->call,(const uint8_t *)civil,strlen(civil),&present,&number),"already in use");assert(present==0 && number==0);
    state->nested+=6;output->data=(const uint8_t *)"<g/>";output->len=4;return 0;
}
static uint32_t clock_now(void *value,double *output) { ((State *)value)->clocks++;*output=42.875;return 0; }
static uint32_t calendar(void *value,uint32_t operation,const uint8_t *input,size_t count,PeleBorrowedBytes *output) {
    State *state=value;assert(input && count);
    const char *reply;
    if(operation==0) { state->calendars++;assert(count==5 && memcmp(input,"123.0",5)==0);
        reply="{\"civil\":{\"year\":2024,\"month\":1,\"day\":2,\"hour\":0,\"minute\":0,\"second\":0,\"millisecond\":0},\"weekday\":2,\"utcOffsetMinutes\":330}";
    } else if(operation==1) { state->instants++;reply="-0.875"; }
    else { assert(operation==2);state->dates++;reply="123.875"; }
    output->data=(const uint8_t *)reply;output->len=strlen(reply);return 0;
}
static void render(void *value,void *model,PeleForeignCallV10 *call,const PeleProviderRenderV10 *request,PeleForeignReplyV10 *reply) {
    (void)request;assert(value==model);State *state=value;state->call=call;
    const char *name="ﬂ°°65¶ß";
    for(int i=0;i<2;i++) { PeleResult *result=pele_foreign_icon_v10(call,(const uint8_t *)name,strlen(name));assert(contains(result,"<g/>"));success(result); }
    uint64_t number=123;uint32_t present=123;
    success(pele_foreign_now_millis_v10(call,&number));assert(number==bits(42.875));
    success(pele_foreign_now_millis_v10(call,&number));assert(number==bits(42.875));
    success(pele_foreign_parse_date_v10(call,(const uint8_t *)"fraction",8,&present,&number));assert(present==1 && number==bits(123));
    PeleResult *fields=pele_foreign_calendar_fields_v10(call,bits(123.875));assert(contains(fields,"330"));success(fields);
    fields=pele_foreign_calendar_fields_v10(call,bits(NAN));assert(contains(fields,"null"));success(fields);
    const char *civil="{\"year\":2024,\"month\":1,\"day\":2,\"hour\":0,\"minute\":0,\"second\":0,\"millisecond\":0}";
    success(pele_foreign_calendar_instant_v10(call,(const uint8_t *)civil,strlen(civil),&present,&number));assert(present==1 && number==bits(-0.0));
    number=123;failure(pele_foreign_parse_date_v10(call,(const uint8_t *)"x",1,NULL,&number),"presence output is null");assert(number==0);
    present=123;failure(pele_foreign_calendar_instant_v10(call,(const uint8_t *)civil,strlen(civil),&present,NULL),"number output is null");assert(present==0);
    reply->payload=owned("<svg xmlns=\"http://www.w3.org/2000/svg\"><g/></svg>");reply->width_bits=bits(20);reply->height_bits=bits(10);state->call=NULL;
}
int main(void) {
    State state={0};PeleRegistry *registry=NULL;success(pele_registry_create_v8(0,&registry));
    PeleDiagramProviderV10 provider={0};provider.version=10;provider.struct_size=sizeof provider;provider.context=&state;
    provider.retain=retain;provider.release=release;provider.release_model=release_model;provider.describe=describe;provider.parse=parse;provider.render=render;
    success(pele_registry_register_provider_v10(registry,(const uint8_t *)"pie",3,&provider));
    PeleProviderDocument *document=NULL;
    success(pele_registry_parse_provider_v10(registry,(const uint8_t *)"pie",3,NULL,0,0,0,0,0,&state,&document));assert(document);
    PeleHostResourcesV3 host={0};host.base.context=&state;host.base.icon=icon;host.base.now=clock_now;
    host.calendar=calendar;host.calendar_instant=calendar;host.date_parser=calendar;
    PeleResult *result=pele_provider_render_v10(document,0,NULL,0,NULL,0,0,&host,&state);
    assert(contains(result,"\"Ok\"") && contains(result,"<svg"));success(result);
    assert(state.icons==2 && state.clocks==1 && state.calendars==1 && state.dates==1 && state.instants==1 && state.nested==12);
    pele_registry_free_v8(registry);assert(state.refs==1);pele_provider_document_free_v10(document);assert(state.refs==0);
    uint64_t number=123;failure(pele_foreign_now_millis_v10(NULL,&number),"Foreign call is null");assert(number==0);
    puts("foreign provider resources, session policies and cross-resource reentry passed");return 0;
}
