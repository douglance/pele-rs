#include "pele.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct { int refs, parses, renders, models, drops, keep_comments, generation, throw_render, malformed_parse; PeleRegistry *registry; } Provider;
typedef struct { double width; int renders; } Model;
static uint64_t bits(double v) { uint64_t b; memcpy(&b,&v,sizeof b); return b; }
static void release_bytes(void *p) { free(p); }
static PeleForeignBytesV10 owned(const char *text) {
    size_t n=strlen(text); char *copy=malloc(n+1); assert(copy); memcpy(copy,text,n+1);
    return (PeleForeignBytesV10){copy,(const uint8_t *)copy,n,release_bytes};
}
static int contains(PeleResult *r,const char *text) {
    size_t n=pele_result_len(r); char *copy=malloc(n+1); assert(copy); memcpy(copy,pele_result_data(r),n);copy[n]=0;
    int yes=strstr(copy,text)!=NULL;free(copy);return yes;
}
static void success(PeleResult *r) { assert(r && pele_result_status(r)==0);pele_result_free(r); }
static void retain_provider(void *c) { ((Provider *)c)->refs++; }
static void release_provider(void *c) { Provider *p=c;assert(p->refs>0);p->refs--; }
static void release_model(void *c,void *m) { Provider *p=c;p->drops++;p->models--;free(m); }
static void describe(void *c,PeleForeignCallV10 *call,uint32_t phase,PeleForeignReplyV10 *out) {
    (void)call;(void)phase;Provider *p=c;
    out->payload=owned(p->keep_comments ? "{\"keepComments\":true,\"section\":\"prototype\"}" : "{\"keepComments\":false,\"section\":\"prototype\"}");
}
static void parse(void *c,PeleForeignCallV10 *call,const PeleProviderParseV10 *r,PeleForeignReplyV10 *out) {
    (void)call;Provider *p=c;p->parses++;
    assert(r->source_len && r->source[r->source_len-1]=='\n');
    assert(r->edge_kind==2 && r->edge_bits==bits(5000));
    assert(r->has_title==1 && r->title_len==5 && memcmp(r->title,"Title",5)==0);
    Model *m=malloc(sizeof *m);assert(m);*m=(Model){40,0};p->models++;out->model=m;
    if(p->malformed_parse) { out->status=1;out->payload=owned("{}"); }
}
static void render(void *c,void *model,PeleForeignCallV10 *call,const PeleProviderRenderV10 *r,PeleForeignReplyV10 *out) {
    Provider *p=c;Model *m=model;p->renders++;m->renders++;
    assert(r->config_len>0);assert(pele_foreign_call_context_v10(call)!=NULL);
    if(p->registry) {
        PeleResult *busy=pele_registry_registered_v8(p->registry,(const uint8_t *)"pie",3);
        /* Rendering a retained document doesn't lock its former Registry. */
        success(busy);
    }
    if(p->throw_render) { out->status=2;out->exception=7;return; }
    char svg[200];snprintf(svg,sizeof svg,"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"%.0f\" height=\"12\"><rect data-generation=\"%d\" width=\"%.0f\" height=\"12\"/></svg>",m->width,p->generation,m->width);
    out->payload=owned(svg);out->width_bits=bits(m->width);out->height_bits=bits(12);
}
static PeleDiagramProviderV10 table(Provider *p) {
    PeleDiagramProviderV10 v={0};v.version=10;v.struct_size=sizeof v;v.context=p;
    v.retain=retain_provider;v.release=release_provider;v.release_model=release_model;v.describe=describe;v.parse=parse;v.render=render;return v;
}
static PeleProviderDocument *parsed(PeleRegistry *r) {
    const char *source="---\ntitle: Title\nconfig:\n  prototype:\n    useMaxWidth: false\n---\npie\n%% kept when requested\n\"A\":1";
    PeleProviderDocument *doc=NULL;int invocation=0;
    PeleResult *result=pele_registry_parse_provider_v10(r,(const uint8_t *)source,strlen(source),NULL,0,0,0,0,0,&invocation,&doc);
    assert(contains(result,"\"Ok\""));success(result);assert(doc);return doc;
}
int main(void) {
    PeleRegistry *registry=NULL;success(pele_registry_create_v8(0,&registry));
    Provider a={0},b={0};a.generation=1;b.generation=2;
    PeleDiagramProviderV10 av=table(&a),bv=table(&b);
    PeleDiagramProviderV10 invalid=av;invalid.struct_size--;
    PeleResult *bad=pele_registry_register_provider_v10(registry,(const uint8_t *)"pie",3,&invalid);
    assert(pele_result_status(bad)!=0 && a.refs==0);pele_result_free(bad);
    success(pele_registry_register_provider_v10(registry,(const uint8_t *)"pie",3,&av));assert(a.refs==1);
    PeleResult *registered=pele_registry_registered_v8(registry,(const uint8_t *)"pie",3);assert(contains(registered,"true"));success(registered);
    PeleProviderDocument *one=parsed(registry);assert(a.models==1);
    a.malformed_parse=1;
    const char *invalid_source="---\ntitle: Title\n---\npie";
    PeleProviderDocument *unpublished=(PeleProviderDocument *)(uintptr_t)1;int pending=0;
    bad=pele_registry_parse_provider_v10(registry,(const uint8_t *)invalid_source,strlen(invalid_source),NULL,0,0,0,0,0,&pending,&unpublished);
    assert(pele_result_status(bad)!=0 && unpublished==NULL && a.models==1 && a.drops==1);pele_result_free(bad);
    a.malformed_parse=0;
    success(pele_registry_register_provider_v10(registry,(const uint8_t *)"pie",3,&bv));assert(a.refs==1 && b.refs==1);
    PeleProviderDocument *two=parsed(registry);assert(b.models==1);
    void *model=NULL;success(pele_provider_model_v10(one,&model));Model *m=model;m->width=91;
    a.generation=3;int invocation=0;
    PeleResult *draw=pele_provider_render_v10(one,0,NULL,0,NULL,0,0,NULL,&invocation);
    assert(contains(draw,"generation=\\\"3") && contains(draw,"\"width\":91"));success(draw);assert(m->renders==1);
    a.throw_render=1;
    draw=pele_provider_render_v10(one,1,NULL,0,NULL,0,0,NULL,&invocation);
    assert(contains(draw,"hostException") && contains(draw,"\"revision\":\"2\""));success(draw);assert(m->renders==2);
    a.throw_render=0;
    draw=pele_provider_render_v10(one,2,NULL,0,NULL,2,bits(0),NULL,&invocation);
    assert(contains(draw,"Diagram output is longer") && contains(draw,"\"revision\":\"3\""));success(draw);assert(m->renders==3);
    draw=pele_provider_render_v10(one,0,NULL,0,NULL,0,0,NULL,&invocation);assert(pele_result_status(draw)!=0);pele_result_free(draw);assert(m->renders==3);
    success(pele_registry_register_builtin_v8(registry,(const uint8_t *)"pie",3));assert(b.refs==1);
    pele_registry_free_v8(registry);
    draw=pele_provider_render_v10(two,0,NULL,0,NULL,0,0,NULL,&invocation);assert(contains(draw,"generation=\\\"2"));success(draw);
    pele_provider_document_free_v10(two);assert(b.refs==0 && b.models==0 && b.drops==1);
    pele_provider_document_free_v10(one);assert(a.refs==0 && a.models==0 && a.drops==2);
    puts("foreign provider ownership/render feature passed");return 0;
}
