
export class ServiceRequest {

    constructor(
        readonly sender: string,
        private readonly dataJson: string,
    ) {
    }

    get data(): any {
        return JSON.parse(this.dataJson);
    }

}

export class ServiceResponse {
    
    constructor(
        readonly success: boolean,
        readonly data: any,
    ) {
    }

    get dataJson(): string {
        return JSON.stringify(this.data);
    }

}