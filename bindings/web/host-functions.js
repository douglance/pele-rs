// Ordinary caller closures stay JavaScript. Only changed records cross the
// borrowed Rust arena; scheduling, geometry and rendering remain native.
export function modelHostFunctions(bridge, operationError, takeResourceError = () => null) {
  return (reference, receiverJSON, argumentsJSON, invokeNative, initialJSON) => {
    if(initialJSON)bridge.reconcileDelta(JSON.parse(initialJSON));
    const previous=bridge.activeInvoke;let invokingNative=false;
    bridge.activeInvoke=(method,receiver,argumentsValue)=>{
      if(invokingNative)throw new TypeError("Active model is already in use");
      invokingNative=true;try {
      const baseline=bridge.graph;
      const input=bridge.snapshot([receiver,...argumentsValue],true);
      const reply=JSON.parse(invokeNative(JSON.stringify({delta:input.delta,
        call:{method,receiver:input.values[0],arguments:input.values.slice(1)}})));
      if(!reply.accepted)bridge.graph=baseline;
      bridge.reconcileDelta(reply.delta);
      const resourceError=takeResourceError();
      if(resourceError)throw resourceError.error;
      if(Object.hasOwn(reply.result,'Err'))throw operationError(reply.result.Err,bridge);
      return bridge.value(reply.result.Ok);
      } finally {invokingNative=false;}
    };
    let value, error, failed=false;
    try {
      const callable=bridge.hostValues[reference];
      if(typeof callable!=='function')throw new TypeError('Host reference is not callable');
      value=Reflect.apply(callable,bridge.value(JSON.parse(receiverJSON)),JSON.parse(argumentsJSON).map(atom=>bridge.value(atom)));
    } catch(thrown) { failed=true;error=thrown; }
    finally {bridge.activeInvoke=previous;}
    const state=bridge.snapshot(failed?[]:[value],true);
    return JSON.stringify({delta:state.delta,result:failed?{Err:bridge.hostException(error)}:{Ok:state.values[0]}});
  };
}
