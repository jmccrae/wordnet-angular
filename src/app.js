angular.module('app', ['ngMaterial']);

angular.module('app').component('wordnet', {
    templateUrl: 'static/wordnet.html',
    controller: function($scope) {
        $scope.synsets = [];
        $scope.display = {
            show_wn31: true
        };
    }
});

angular.module('app').controller('SearchController',
        function($scope, $http) {
    var self = this;
    self.selectedItemChange = function(item) {
        if(item) {
            $http.get("/json/lemma/"+item.item).then(
                function(result) {
                    $scope.$parent.synsets = result.data;
                }, function(response) {
                    console.log(response.data);
                });
        } else {
            $scope.$parent.synsets = [];
        }
    };

    self.querySearch = function (query) {
        return $http.get("/autocomplete/lemma/"+ query).then(
            function(result) {
                return result.data;
            }, function(response) {
                console.log(response.data);
            });
        };
});

angular.module('app').component('synset', {
        templateUrl: '/static/synset.html',
        bindings: {
            synset: '=',
            display: '<'
        }
    });

angular.module('app').component('relation', {
        templateUrl: '/static/relation.html',
        bindings: {
            fullname: '@',
            relation: '@',
            relations: '=',
            display: '<'
        }
    });


angular.module('app').filter('isAdjective', function() {
    return function(items) {
        var filtered = [];
        for(var i = 0; i < items.length; i++) {
            var item = items[i];
            if(item.pos === 'a' || item.pos === 's') {
                filtered.push(item);
            }
        }
        return filtered;
    }
});
