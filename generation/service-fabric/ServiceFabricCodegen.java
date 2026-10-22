import java.util.ArrayList;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;
import org.openapitools.codegen.CodegenDiscriminator;
import org.openapitools.codegen.CodegenModel;
import org.openapitools.codegen.CodegenOperation;
import org.openapitools.codegen.CodegenParameter;
import org.openapitools.codegen.CodegenProperty;
import org.openapitools.codegen.languages.RustClientCodegen;
import org.openapitools.codegen.model.ModelMap;
import org.openapitools.codegen.model.ModelsMap;
import org.openapitools.codegen.model.OperationsMap;

/** Service Fabric wire compatibility, applied to codegen metadata, not the input specification. */
public class ServiceFabricCodegen extends RustClientCodegen {
    @Override
    public Map<String, ModelsMap> postProcessAllModels(Map<String, ModelsMap> models) {
        models = super.postProcessAllModels(models);
        for (ModelsMap entry : models.values()) {
            for (ModelMap modelMap : entry.getModels()) {
                CodegenModel base = modelMap.getModel();
                CodegenDiscriminator discriminator = base.discriminator;
                if (discriminator == null) {
                    continue;
                }
                // Inline the complete subtype, excluding the tag supplied by serde.
                discriminator.getVendorExtensions().put("x-rust-inline-variants", true);
                var mappings = new LinkedHashSet<CodegenDiscriminator.MappedModel>();
                for (var mapped : discriminator.getMappedModels()) {
                    CodegenModel child = mapped.getModel();
                    if (child == null) {
                        throw new IllegalStateException("Missing discriminator model: " + mapped.getModelName());
                    }
                    String wireValue = (String) child.vendorExtensions.getOrDefault(
                        "x-ms-discriminator-value", mapped.getMappingName());
                    mappings.add(new SubtypeMapping(wireValue, mapped,
                        discriminator.getPropertyBaseName(), toModelFilename(child.name)));
                }
                discriminator.setMappedModels(mappings);
            }
        }
        return models;
    }

    /** Shadows the enclosing base model's vars in the standard Rust model template. */
    public static class SubtypeMapping extends CodegenDiscriminator.MappedModel {
        private final List<CodegenProperty> vars;

        public SubtypeMapping(String wireValue, CodegenDiscriminator.MappedModel mapped, String tag, String module) {
            super(wireValue, mapped.getModelName(), mapped.getSchemaName(), mapped.isExplicitMapping());
            setModel(mapped.getModel());
            vars = mapped.getModel().allVars.stream()
                .filter(property -> !tag.equals(property.baseName))
                .map(property -> {
                    if (property.isEnum) {
                        property = property.clone();
                        property.enumName = "models::" + module + "::" + property.enumName;
                    }
                    return property;
                })
                .collect(Collectors.toList());
        }

        public List<CodegenProperty> getVars() {
            return vars;
        }
    }

    @Override
    public OperationsMap postProcessOperationsWithModels(OperationsMap operations, List<ModelMap> models) {
        operations = super.postProcessOperationsWithModels(operations, models);
        for (CodegenOperation operation : operations.getOperations().getOperation()) {
            for (CodegenParameter parameter : operation.allParams) {
                if (parameter.isQueryParam && parameter.baseName.endsWith("HealthStateFilter")) {
                    parameter.dataType = "u32";
                }
            }
            if (operation.returnType != null) {
                List<String> emptySuccessCodes = operation.responses.stream()
                    .filter(response -> response.is2xx && response.dataType == null)
                    .map(response -> response.code)
                    .collect(Collectors.toList());
                if (!emptySuccessCodes.isEmpty()) {
                    operation.vendorExtensions.put("x-sf-optional-response", true);
                    operation.vendorExtensions.put("x-sf-empty-success-codes", emptySuccessCodes);
                }
            }
            // The official upload operations describe raw data but omit a body parameter.
            if ("upload_file".equals(operation.operationId) || "upload_file_chunk".equals(operation.operationId)) {
                CodegenParameter body = new CodegenParameter();
                body.baseName = "body";
                body.paramName = "body";
                body.dataType = "Vec<u8>";
                body.required = true;
                body.isBodyParam = true;
                body.isPrimitiveType = true;
                postProcessParameter(body);
                operation.allParams = new ArrayList<>(operation.allParams);
                operation.allParams.add(body);
                operation.vendorExtensions.put("x-sf-binary-body", true);
            }
        }
        return operations;
    }
}
