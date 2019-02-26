import * as fbs from "./msg_generated";
import * as flatbuffers from "./flatbuffers"
import { sendSync } from "./bridge";
import { URL } from "./url";

export interface RuntimeInfo {
    name: string;
    version: string;
    default_module_url: string;
}

export function getRuntimeInfo(runtimeUuid: string): RuntimeInfo {
    const fbb = flatbuffers.createBuilder();

    const runtimeUuidString = fbb.createString(runtimeUuid);

    fbs.GetRuntimeInfo.startGetRuntimeInfo(fbb);
    fbs.GetRuntimeInfo.addRuntimeUuid(fbb, runtimeUuidString);

    const resp = sendSync(fbb, fbs.Any.RequestServiceRequest, fbs.GetRuntimeInfo.endGetRuntimeInfo(fbb));

    const msg = new fbs.GetRuntimeInfoResp();
    // Write message data to handle
    resp.msg(msg);

    return {
        name: msg.name(),
        version: msg.version(),
        default_module_url: msg.defaultModuleUrl(),
    };
}

export interface ReadFileUrlExists {
    exists: true;
    content: string;
}

export interface ReadFileUrlEmpty {
    exists: false;
}

export type ReadFileUrlResponse = ReadFileUrlExists | ReadFileUrlEmpty;

export function readFileUrl(url: URL): ReadFileUrlResponse {
    const fbb = flatbuffers.createBuilder();

    const urlString = fbb.createString(url.toString());

    fbs.ReadFileUrl.startReadFileUrl(fbb);
    fbs.ReadFileUrl.addUrl(fbb, urlString);

    const resp = sendSync(fbb, fbs.Any.ReadFileUrl, fbs.ReadFileUrl.endReadFileUrl(fbb));

    const msg = new fbs.ReadFileUrlResp();

    resp.msg(msg);

    // Type assertions are weird for this one.
    if (msg.exists()) {
        return {
            exists: true, 
            content: msg.content(),
        };
    } else {
        return {
            exists: false,
        }
    }
}