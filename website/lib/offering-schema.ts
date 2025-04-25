// JSON Schema for Offerings
export const offeringJsonSchema = {
    $schema: "http://json-schema.org/draft-07/schema#",
    type: "object",
    required: ["api_version", "kind", "metadata", "provider", "defaults", "regions"],
    properties: {
        api_version: {
            type: "string",
            description: "The API version of the offering schema"
        },
        kind: {
            type: "string",
            enum: ["Offering"],
            description: "The type of resource, must be 'Offering'"
        },
        metadata: {
            type: "object",
            required: ["name", "version"],
            properties: {
                name: {
                    type: "string",
                    description: "Name of the offering"
                },
                version: {
                    type: "string",
                    description: "Version of the offering"
                }
            }
        },
        provider: {
            type: "object",
            required: ["name"],
            properties: {
                name: {
                    type: "string",
                    description: "Provider name"
                },
                description: {
                    type: "string",
                    description: "Provider description"
                }
            }
        },
        defaults: {
            type: "object",
            required: ["machine_spec"],
            properties: {
                machine_spec: {
                    type: "object",
                    required: ["instance_types"],
                    properties: {
                        instance_types: {
                            type: "array",
                            items: {
                                type: "object",
                                required: ["id", "cpu", "memory", "storage", "pricing"],
                                properties: {
                                    id: {
                                        type: "string",
                                        description: "Instance type identifier"
                                    },
                                    type: {
                                        type: "string",
                                        description: "Instance category"
                                    },
                                    cpu: {
                                        type: "string",
                                        description: "CPU specification"
                                    },
                                    memory: {
                                        type: "string",
                                        description: "Memory specification"
                                    },
                                    storage: {
                                        type: "object",
                                        required: ["type", "size"],
                                        properties: {
                                            type: {
                                                type: "string",
                                                description: "Storage type"
                                            },
                                            size: {
                                                type: "string",
                                                description: "Storage size"
                                            }
                                        }
                                    },
                                    pricing: {
                                        type: "object",
                                        description: "Pricing details"
                                    },
                                    metadata: {
                                        type: "object",
                                        description: "Additional metadata"
                                    }
                                }
                            }
                        }
                    }
                },
                terms_of_service: {
                    type: "array",
                    items: {
                        type: "string"
                    },
                    description: "Terms of service"
                },
                network_spec: {
                    type: "object",
                    description: "Network specifications"
                }
            }
        },
        regions: {
            type: "array",
            items: {
                type: "object",
                required: ["name", "geography"],
                properties: {
                    name: {
                        type: "string",
                        description: "Region name"
                    },
                    description: {
                        type: "string",
                        description: "Region description"
                    },
                    geography: {
                        type: "object",
                        required: ["continent", "country"],
                        properties: {
                            continent: {
                                type: "string",
                                description: "Continent name"
                            },
                            country: {
                                type: "string",
                                description: "Country name"
                            },
                            iso_codes: {
                                type: "object",
                                description: "ISO codes for region"
                            }
                        }
                    },
                    availability_zones: {
                        type: "array",
                        items: {
                            type: "object",
                            required: ["name"],
                            properties: {
                                name: {
                                    type: "string",
                                    description: "Availability zone name"
                                },
                                description: {
                                    type: "string",
                                    description: "Availability zone description"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
};
