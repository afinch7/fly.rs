
export class ServiceRequest {

    constructor(
        readonly action: string,
        private readonly dataJson: string,
    ) {
    }

    get data(): any {
        return JSON.parse(this.dataJson);
    }

}

export class ServiceResponse {
    
    constructor(
        readonly data: any,
    ) {
    }

    get dataJson(): string {
        return JSON.stringify(this.data);
    }

}