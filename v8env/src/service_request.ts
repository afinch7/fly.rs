import * as fbs from "./msg_generated";
import * as flatbuffers from "./flatbuffers"
import { sendSync } from "./bridge";

export interface ServiceResponse {
    success: boolean;
    data: any;
}

/**
 * 
 * @param destinationName The `ServiceName` of the service you want to send this to.
 * @param data The data retured by said service.
 */
export function serviceRequest(destinationName: string, data: any): ServiceResponse {
    const fbb = flatbuffers.createBuilder();

    const destinationNameString = fbb.createString(destinationName);
    const dataString = fbb.createString(JSON.stringify(data));

    fbs.RequestServiceRequest.startRequestServiceRequest(fbb);
    fbs.RequestServiceRequest.addDestinationName(fbb, destinationNameString);
    fbs.RequestServiceRequest.addData(fbb, dataString);

    const resp = sendSync(fbb, fbs.Any.RequestServiceRequest, fbs.RequestServiceRequest.endRequestServiceRequest(fbb));

    const msg = new fbs.RequestServiceResponse();
    // Write message data to handle
    resp.msg(msg);

    return {
        success: msg.success(),
        data: JSON.parse(msg.data()),
    }
}